use std::env;
use sqlx::sqlite;

mod downloader;
pub mod rss;

pub async fn get_pool() -> anyhow::Result<sqlite::SqlitePool> {
    dotenv::dotenv().ok();

    Ok(sqlite::SqlitePool::connect(&env::var("DATABASE_URL")?).await?)
}
