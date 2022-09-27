use axum::{
    extract,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDate;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqlitePool, Connection};
use std::{env, net::SocketAddr};
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

async fn create_order(
    extract::Json(payload): extract::Json<OrderInput>,
    Extension(db): Extension<DB>,
) -> Result<impl IntoResponse, AppError> {
    let DB(pool) = db;

    let OrderInput {
        rooms_order,
        address_details,
    } = payload;

    let mut tx = pool.acquire().await?;

    let order_id = tx.transaction::<_, _, anyhow::Error>(|conn| Box::pin(async move {
        let billing_street_add = address_details.billing_street_add.unwrap_or_else(|| String::from(""));

        let customer_id = sqlx::query!("INSERT INTO customers (email, billing_street, billing_street_add, billing_city, billing_postcode, billing_country) VALUES ($1, $2, $3, $4, $5, $6)",
            address_details.email,
            address_details.billing_street,
            billing_street_add,
            address_details.billing_city,
            address_details.billing_postcode,
            address_details.billing_country
        ).execute(&mut *conn).await?.last_insert_rowid();

        let order_id = sqlx::query!("INSERT INTO orders (customer_id, stripe_intent_id) VALUES ($1, $2)", customer_id, "").execute(&mut *conn).await?.last_insert_rowid();

        for order in rooms_order {
            let start_date = order.start_date.to_string();
            let end_date = order.end_date.to_string();

            sqlx::query!("INSERT INTO order_items (room_id, order_id, start_date, end_date) VALUES($1, $2, $3, $4)", order.room_id, order_id, start_date, end_date).execute(&mut *conn).await?;
        }

        Ok(order_id)
    })).await?;

    Ok(Json(json!({ "order_id": order_id })))
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    println!("{:?}", "2022-07-11".parse::<NaiveDate>());

    let db = DB(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    let app = Router::new()
        .route("/", get(root))
        .route("/rooms", get(list_rooms))
        .route("/order", post(create_order))
        .layer(Extension(db))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server starting on port {}", 3000);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
