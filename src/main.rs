use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, routing::get, Extension, Json, Router};
use chrono::NaiveDate;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::sqlite::SqlitePool;
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

#[derive(Deserialize, Serialize)]
struct RoomOrder {
    room_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Deserialize, Serialize)]
struct OrderAddress {
    email: String,
    billing_street: String,
    billing_street_add: Option<String>,
    billing_city: String,
    billing_postcode: String,
    billing_country: String,
}

#[derive(Deserialize, Serialize)]
struct OrderInput {
    rooms_order: Vec<RoomOrder>,
    address_details: OrderAddress,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db = DB(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    let app = Router::new()
        .route("/", get(root))
        .route("/rooms", get(list_rooms))
        .layer(Extension(db))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server starting on port {}", 3000);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
