use anyhow::Result;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let db = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&db).await?;
    Ok(())
}
