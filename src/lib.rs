use dotenvy::dotenv;
use downloader::{DownloadManager, DownloadingTorrent};
use log::{debug, error};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use crate::rss::parsers;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tokio::sync::{Mutex, OnceCell};

pub mod downloader;
pub mod notification;
pub mod renamer;
pub mod rss;
#[cfg(test)]
mod test;

macro_rules! log_with {
    ($level:ident, $prefix:expr, $($arg:tt)*) => {
        log::$level!("[{:04}]{}", $prefix, format_args!($($arg)*))
    };
}

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

pub async fn download_rss_feeds(downloader: &DownloadManager) -> BTResult<()> {
    debug!("[rss] Fetching RSS feeds...");
    let rss_list = rss::store::query_rss().await.unwrap_or_default();

    for rss in rss_list {
        let rss_id = rss.id.expect("Rss id should not be None here.");
        if !rss.enabled.unwrap_or(false) {
            log_with!(debug, rss_id, "[rss] Skip disabled RSS: ({})", rss.url);
            continue;
        }

        match parsers::parse(&rss).await {
            Ok(feeds) => {
                for feed in &feeds.items {
                    // TODO: Rss filter should contains including type.
                    // If the torrent files mismatch the filter rules, skip downloading
                    if let Some(filter) = &rss.filters {
                        if !filter.is_match(&feed).await {
                            log_with!(info, rss_id, "[parser] Skip torrent by rules: {:?}", feed);
                            continue;
                        }
                    }

                    downloader
                        .download_with_state(Some(rss_id), &feed.torrent, &feed.into())
                        .await
                        .unwrap_or_else(|e| {
                            log_with!(
                                error,
                                rss_id,
                                "[parser] Failed to download torrent: {:?}",
                                e
                            );
                        });
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
            Err(e) => {
                log_with!(error, rss_id, "[parser] Failed to parse RSS: {:?}", e);
            }
        }
    }
    Ok(())
}

pub async fn rename_downloaded_files(
    download_task: &DownloadingTorrent,
    archived_path: &str,
    download_path_mapping: Option<&str>,
    notifier: Option<Arc<Mutex<Box<dyn notification::Notifier>>>>,
) -> BTResult<()> {
    let dst_folder = Path::new(archived_path);
    let torrent_hash = &download_task.hash;

    // ignore all tasks renamed or not found
    match downloader::store::is_renamed(&torrent_hash).await {
        Ok(true) => {
            debug!("[rename] Skip renaming task [{}] already renamed", torrent_hash);
            return Ok(());
        }
        Err(_) => {
            debug!("[rename] Skip renaming task download manually: [{}]", torrent_hash);
            return Ok(());
        }
        _ => {}
    };

    let src_path = PathBuf::from(&download_task.save_path).join(&download_task.name);
    let mut remapped_src_path = src_path.clone();

    // replace path if `download_path_mapping` is set
    if let Some(path_map) = download_path_mapping.as_ref() {
        remapped_src_path = renamer::replace_path(src_path.clone(), path_map);
    }

    match downloader::store::get_bangumi_info(&torrent_hash).await? {
        Some(info) => {
            match renamer::rename(&info, &remapped_src_path, dst_folder) {
                Ok(()) => {
                    downloader::store::update_task_renamed(&torrent_hash).await?;

                    // Send notification
                    if let Some(notifier) = notifier.as_ref() {
                        let msg = notification::Notification::DownloadFinished(info).to_string();
                        let notifier_lock = notifier.lock().await;
                        debug!("[notification] Sending notification: {}", msg);
                        notifier_lock.send(&msg).await;
                    }
                }
                Err(e) => {
                    error!("[rename] Failed to rename task [{}]: {:?}", torrent_hash, e);
                }
            }
        }
        None => {
            // This task was downloaded manually.
            debug!("[rename] Skip renaming task [{}] without media info", torrent_hash);
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

    #[error("Parsing error: {0}")]
    ParsingError(#[from] parsers::ParsingError),
}

pub type BTResult<T> = Result<T, BTError>;
