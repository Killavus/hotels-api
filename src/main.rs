use anyhow::Result;
use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use dotenv::dotenv;
use serde_json::json;
use sqlx::sqlite::SqlitePool;
use std::{env, net::SocketAddr};

#[derive(Clone, Debug)]
struct DB(SqlitePool);

async fn root() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    let db = DB(SqlitePool::connect(&env::var("DATABASE_URL")?).await?);

    let app = Router::new()
        .layer(Extension(db.clone()))
        .route("/", get(root));

    sqlx::migrate!().run(&db.0).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server starting on port {}", 3000);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
