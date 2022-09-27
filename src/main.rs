use axum::{
    extract::{self, Path},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDate;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqlitePool, Connection, Sqlite};
use std::{env, net::SocketAddr};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone, Debug)]
struct DB(SqlitePool);

async fn root() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

struct RoomDSO {
    id: i64,
    name: String,
    beds: i64,
    pets_allowed: bool,
    price_in_cents: i64,
    hotel_name: String,
    hotel_id: i64,
}

#[derive(Serialize)]
struct RoomResponse {
    id: i64,
    name: String,
    beds: i64,
    pets_allowed: bool,
    price_in_cents: i64,
    hotel: HotelResponse,
}

#[derive(Serialize)]
struct HotelResponse {
    id: i64,
    name: String,
}

impl From<RoomDSO> for RoomResponse {
    fn from(dso: RoomDSO) -> Self {
        Self {
            id: dso.id,
            name: dso.name,
            beds: dso.beds,
            pets_allowed: dso.pets_allowed,
            price_in_cents: dso.price_in_cents,
            hotel: HotelResponse {
                id: dso.hotel_id,
                name: dso.hotel_name,
            },
        }
    }
}

async fn list_rooms(Extension(db): Extension<DB>) -> impl IntoResponse {
    let DB(pool) = db;
    let records = sqlx::query_as!(RoomDSO,
        "SELECT rooms.id, rooms.name, beds, pets_allowed, price_in_cents, hotels.name AS hotel_name, hotel_id AS hotel_id FROM rooms INNER JOIN hotels ON hotels.id = rooms.hotel_id"
    ).fetch_all(&pool).await;

    if records.is_err() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "failed to fetch rooms" })),
        )
    } else {
        let records = records.unwrap();

        (
            StatusCode::OK,
            Json(
                json!({ "rooms": records.into_iter().map(RoomResponse::from).collect::<Vec<_>>() }),
            ),
        )
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct RoomOrder {
    room_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Deserialize, Serialize, Debug)]
struct OrderAddress {
    email: String,
    billing_street: String,
    billing_street_add: Option<String>,
    billing_city: String,
    billing_postcode: String,
    billing_country: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct OrderInput {
    rooms_order: Vec<RoomOrder>,
    address_details: OrderAddress,
}

#[derive(Debug)]
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::warn!("Error while processing request: {}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

async fn persist_order(
    pool: impl sqlx::Acquire<'_, Database = Sqlite>,
    rooms_order: Vec<RoomOrder>,
    address_details: OrderAddress,
) -> Result<i64, AppError> {
    let mut tx = pool.acquire().await?;

    tx.transaction::<_, _, anyhow::Error>(|conn| Box::pin(async move {
        let billing_street_add = address_details.billing_street_add.unwrap_or_else(|| String::from(""));

        let customer_id = sqlx::query!("INSERT INTO customers (email, billing_street, billing_street_add, billing_city, billing_postcode, billing_country) VALUES ($1, $2, $3, $4, $5, $6)",
            address_details.email,
            address_details.billing_street,
            billing_street_add,
            address_details.billing_city,
            address_details.billing_postcode,
            address_details.billing_country
        ).execute(&mut *conn).await?.last_insert_rowid();

        let order_id = sqlx::query!("INSERT INTO orders (customer_id) VALUES ($1)", customer_id).execute(&mut *conn).await?.last_insert_rowid();

        for order in rooms_order {
            let start_date = order.start_date.to_string();
            let end_date = order.end_date.to_string();

            sqlx::query!("INSERT INTO order_items (room_id, order_id, start_date, end_date) VALUES($1, $2, $3, $4)", order.room_id, order_id, start_date, end_date).execute(&mut *conn).await?;
        }

        Ok(order_id)
    })).await.map_err(AppError)
}

async fn calculate_order_price(
    tx: impl sqlx::Executor<'_, Database = Sqlite> + Copy,
    order_id: i64,
) -> Result<i64, AppError> {
    let order_with_price_details = sqlx::query!(
        "
      SELECT
        order_items.start_date AS start_date,
        order_items.end_date AS end_date,
        rooms.price_in_cents AS price
      FROM orders
      INNER JOIN order_items ON order_items.id = orders.id
      INNER JOIN rooms ON order_items.room_id = rooms.id
      WHERE orders.id = $1",
        order_id
    )
    .fetch_all(tx)
    .await?;

    Ok(order_with_price_details.into_iter().fold(0, |total, r| {
        let start_date: NaiveDate = r.start_date.parse().unwrap_or(NaiveDate::MIN);
        let end_date: NaiveDate = r.end_date.parse().unwrap_or(NaiveDate::MIN);

        if end_date == NaiveDate::MIN || start_date == NaiveDate::MIN {
            0
        } else {
            let reservation_days: i64 = (end_date - start_date).num_days();

            total + reservation_days * r.price
        }
    }))
}

async fn customer_email(
    tx: impl sqlx::Executor<'_, Database = Sqlite> + Copy,
    order_id: i64,
) -> Result<String, AppError> {
    let customer_email_record = sqlx::query!(
        "SELECT email FROM customers INNER JOIN orders ON orders.id = $1",
        order_id
    )
    .fetch_one(tx)
    .await?;

    Ok(customer_email_record.email)
}

async fn create_order(
    extract::Json(payload): extract::Json<OrderInput>,
    Extension(db): Extension<DB>,
) -> Result<impl IntoResponse, AppError> {
    let DB(pool) = db;

    let OrderInput {
        rooms_order,
        address_details,
    } = payload;

    let order_id = persist_order(&pool, rooms_order, address_details).await?;

    Ok(Json(json!({ "order_id": order_id })))
}

async fn payment_intent_for_order(
    tx: impl sqlx::Executor<'_, Database = Sqlite> + Copy,
    order_id: i64,
    client: stripe::Client,
) -> Result<stripe::PaymentIntent, AppError> {
    use stripe::{PaymentIntent, PaymentIntentId};

    let payment_intent_id = sqlx::query!("SELECT payment_intent_id FROM order_payments
        INNER JOIN orders ON orders.id = order_payments.order_id WHERE order_payments.order_id = $1", order_id).fetch_optional(tx).await?;

    match payment_intent_id {
        Some(record) => {
            let payment_intent_id = record.payment_intent_id;
            let payment_intent_id = payment_intent_id.parse::<PaymentIntentId>()?;

            let intent = PaymentIntent::retrieve(&client, &payment_intent_id, &[]).await?;
            Ok(intent)
        }
        None => {
            use stripe::{CreatePaymentIntent, Currency};
            let order_price = calculate_order_price(tx, order_id).await?;
            let customer_email = customer_email(tx, order_id).await?;

            let create_payload = {
                let mut intent = CreatePaymentIntent::new(order_price, Currency::PLN);
                intent.receipt_email = Some(&customer_email);
                intent.payment_method_types = Some(vec![String::from("card")]);
                intent
            };

            let payment_intent = PaymentIntent::create(&client, create_payload).await?;
            let payment_intent_id = payment_intent.id.to_string();
            sqlx::query!(
                "INSERT INTO order_payments (order_id, payment_intent_id) VALUES ($1, $2)",
                order_id,
                payment_intent_id
            )
            .execute(tx)
            .await?;

            Ok(payment_intent)
        }
    }
}

async fn create_order_payment_intent(
    Path(id): Path<i64>,
    Extension(DB(pool)): Extension<DB>,
    Extension(StripeClient(stripe)): Extension<StripeClient>,
) -> Result<impl IntoResponse, AppError> {
    let payment_intent = payment_intent_for_order(&pool, id, stripe).await?;

    Ok(Json(
        json!({ "client_secret": payment_intent.client_secret }),
    ))
}

#[derive(Clone)]
struct StripeClient(stripe::Client);

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db = DB(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    let app = Router::new()
        .route("/", get(root))
        .route("/rooms", get(list_rooms))
        .route("/order", post(create_order))
        .route("/order/:id/payment", post(create_order_payment_intent))
        .layer(Extension(db))
        .layer(Extension(StripeClient(stripe::Client::new(env::var(
            "STRIPE_SECRET_KEY",
        )?))))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([Method::GET, Method::POST]),
        );

    let addr: SocketAddr = env::var("APP_LISTEN")
        .unwrap_or_else(|_| String::from("127.0.0.1:9999"))
        .parse()?;

    tracing::info!("Server starting, listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
