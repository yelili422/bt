use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use log::{error, info};

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use crate::downloader::Downloader;
use crate::rss::parsers;

pub mod downloader;
pub mod renamer;
pub mod rss;


pub async fn download_rss_feeds(downloader: Arc<Mutex<Box<dyn Downloader>>>) -> anyhow::Result<()> {
    info!("[rss] Fetching RSS feeds...");
    let pool = crate::get_pool().await?;
    let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
    for rss in rss_list {
        let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
        match parsers::parse(&rss).await {
            Ok(feeds) => {
                for feed in &feeds.items {
                    downloader::download_with_state(
                        downloader.clone(),
                        &feed.torrent,
                        &feed.into(),
                    )
                        .await
                        .unwrap_or_else(|e| {
                            error!("[parser] Failed to download torrent: {:?}", e);
                        });
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
            Err(e) => {
                error!("[parser] Failed to parse RSS: {:?}", e);
            }
        }
    }

    Ok(())
}

pub async fn check_downloading_tasks(
    downloader: Arc<Mutex<Box<dyn Downloader>>>,
    archived_path: String,
    downloading_path_map: Option<String>,
) -> anyhow::Result<()> {
    // update task status
    info!("[downloader] Updating task status...");
    let downloader_lock = downloader.lock().await;
    let download_list = downloader_lock
        .get_download_list()
        .await
        .unwrap_or_default();
    downloader::update_task_status(&download_list).await?;

    // if task is done, rename the file and update the database
    info!("[cmd] Renaming completed tasks...");
    let dst_folder = Path::new(&archived_path);
    for task in download_list {
        if task.status == downloader::TaskStatus::Completed {
            let mut file_path = PathBuf::from(task.save_path).join(task.name);

            if let Some(path_map) = downloading_path_map.as_ref() {
                file_path = renamer::replace_path(file_path, path_map);
            }

            match renamer::rename(
                &downloader::get_bangumi_info(&task.hash).await?,
                &file_path,
                dst_folder,
            ) {
                Ok(_) => {}
                Err(e) => {
                    error!("[cmd] Failed to rename: {:?}", e);
                }
            }
            downloader::set_task_renamed(&task.hash).await?;
        }
    }

    Ok(())
}


pub async fn get_pool() -> anyhow::Result<SqlitePool> {
    // TODO: reuse the pool if it already exists

    let url = &env::var("DATABASE_URL")?;
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

    Ok(SqlitePool::connect_with(options).await?)
}
