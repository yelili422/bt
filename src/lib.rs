use std::env;
use std::str::FromStr;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

pub mod downloader;
pub mod renamer;
pub mod rss;

pub async fn get_pool() -> anyhow::Result<SqlitePool> {
    // TODO: reuse the pool if it already exists

    let url = &env::var("DATABASE_URL")?;
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true);

    Ok(SqlitePool::connect_with(options).await?)
}
