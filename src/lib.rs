use sqlx::sqlite;
use std::env;

mod downloader;
pub mod rss;

pub async fn get_pool() -> anyhow::Result<sqlite::SqlitePool> {
    dotenv::dotenv().ok();

    Ok(sqlite::SqlitePool::connect(&env::var("DATABASE_URL")?).await?)
}
