use dotenvy::dotenv;
use downloader::DownloadingTorrent;
use log::{debug, error, info};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use crate::downloader::Downloader;
use crate::rss::parsers;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Mutex, OnceCell};

pub mod api;
pub mod downloader;
pub mod notification;
pub mod renamer;
pub mod rss;
#[cfg(test)]
mod test;

#[allow(unused)]
static INIT: OnceCell<()> = OnceCell::const_new();

pub async fn init() {
    // We should initialize something only once, but `init` can be called multiple times in tests.
    INIT.get_or_init(|| async {
        // Load environment variables from .env file.
        // If not found, ignore it
        _ = dotenv();

        // Init logger
        env_logger::init();

        let pool = get_pool().await;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run database migrations");
    })
    .await;
}

pub async fn download_rss_feeds(downloader: Arc<Mutex<Box<dyn Downloader>>>) -> BTResult<()> {
    debug!("[rss] Fetching RSS feeds...");
    let rss_list = rss::list_rss().await.unwrap_or_default();

    for rss in rss_list {
        if !rss.enabled.unwrap_or(false) {
            debug!("[rss] Skip disabled RSS: ({})", rss.url);
            continue;
        }
        match parsers::parse(&rss).await {
            Ok(feeds) => {
                for feed in &feeds.items {
                    // Skip downloading if the torrent info already in the database .
                    // Because we don't need to download again if this task was renamed from the downloader.
                    if downloader::is_task_exist(&feed.torrent.url).await? {
                        debug!("[parser] Task already in downloading list: {:?}", feed);
                        continue;
                    }

                    // If the torrent files mismatch the filter rules, skip downloading
                    if let Some(filter) = &rss.filters {
                        if !filter.is_match(&feed).await {
                            info!("[parser] Skip torrent by rules: {:?}", feed);
                            continue;
                        }
                    }

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

pub async fn checking_download_task(
    downloader: Arc<Mutex<Box<dyn Downloader>>>,
    finished_ch: mpsc::Sender<DownloadingTorrent>,
) -> BTResult<()> {
    let downloader_lock = downloader.lock().await;
    let download_list = downloader_lock.get_download_list().await?;
    downloader::update_task_status(&download_list).await?;
    for task in download_list {
        if task.status == downloader::TaskStatus::Completed {
            finished_ch.send(task).await.unwrap();
        }
    }
    Ok(())
}

pub async fn rename_downloaded_files(
    mut finished_ch: mpsc::Receiver<DownloadingTorrent>,
    archived_path: String,
    download_path_mapping: Option<String>,
    notifier: Option<Arc<Mutex<Box<dyn notification::Notifier>>>>,
) -> BTResult<()> {
    let dst_folder = Path::new(&archived_path);
    while let Some(task) = finished_ch.recv().await {
        // ignore all tasks renamed or not found
        match downloader::is_renamed(&task.hash).await {
            Ok(true) => {
                debug!("[rename] Skip renaming task [{}] already renamed", task.hash);
                continue;
            }
            Err(_) => {
                debug!("[rename] Skip renaming task download manually: [{}]", task.hash);
                continue;
            }
            _ => {}
        };

        let src_path = PathBuf::from(task.save_path).join(task.name);
        let mut remapped_src_path = src_path.clone();

        // replace path if `download_path_mapping` is set
        if let Some(path_map) = download_path_mapping.as_ref() {
            remapped_src_path = renamer::replace_path(src_path.clone(), path_map);
        }

        match downloader::get_bangumi_info(&task.hash).await? {
            Some(info) => {
                match renamer::rename(&info, &remapped_src_path, dst_folder) {
                    Ok(()) => {
                        downloader::set_task_renamed(&task.hash).await?;

                        // TODO: Try to move the downloaded files to a separate folder.

                        // Send notification
                        if let Some(notifier) = notifier.as_ref() {
                            let msg =
                                notification::Notification::DownloadFinished(info).to_string();
                            let notifier_lock = notifier.lock().await;
                            debug!("[notification] Sending notification: {}", msg);
                            notifier_lock.send(&msg).await;
                        }
                    }
                    Err(e) => {
                        error!("[rename] Failed to rename task [{}]: {:?}", task.hash, e);
                    }
                }
            }
            None => {
                // This task was downloaded manually.
                debug!("[rename] Skip renaming task [{}] without media info", task.hash);
            }
        }
    }

    Ok(())
}

static SQL_POOL: OnceCell<SqlitePool> = OnceCell::const_new();

type DBError = sqlx::Error;

type DBResult<T> = Result<T, DBError>;

async fn init_db() -> DBResult<SqlitePool> {
    #[cfg(not(test))]
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    #[cfg(test)]
    let url = "sqlite::memory:".to_string();

    let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    Ok(pool)
}

pub async fn get_pool() -> SqlitePool {
    let pool = SQL_POOL
        .get_or_init(|| async { init_db().await.expect("Failed to initialize database") })
        .await;
    pool.clone()
}

pub async fn tx_begin() -> DBResult<sqlx::Transaction<'static, sqlx::Sqlite>> {
    let pool = get_pool().await;
    Ok(pool.begin().await?)
}

#[derive(Debug, thiserror::Error)]
pub enum BTError {
    #[error("Database error: {0}")]
    DBError(#[from] DBError),

    #[error("Downloader error: {0}")]
    DownloaderError(#[from] downloader::DownloaderError),

    #[error("Renamer error: {0}")]
    ParsingError(#[from] parsers::ParsingError),
}

type BTResult<T> = Result<T, BTError>;
