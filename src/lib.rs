use sqlx::sqlite;
use std::env;

pub mod downloader;
pub mod renamer;
pub mod rss;

pub async fn get_pool() -> anyhow::Result<sqlite::SqlitePool> {
    dotenv::dotenv().ok();

    // TODO: reuse the pool if it already exists
    Ok(sqlite::SqlitePool::connect(&env::var("DATABASE_URL")?).await?)
}
