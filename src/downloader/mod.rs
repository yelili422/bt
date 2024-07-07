mod bittorrent;
mod dummy;
mod qbittorrent;
mod store;
mod task;

use async_trait::async_trait;
use log::{debug, error};
use lru::LruCache;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::EnumString;
use tokio::sync::Mutex;
use typed_builder::TypedBuilder;

use crate::renamer::BangumiInfo;
use crate::{tx_begin, DBError, DBResult};
pub use bittorrent::*;
pub use dummy::DummyDownloader;
pub use qbittorrent::QBittorrentDownloader;
pub use task::*;

/// The metadata of a torrent file
#[derive(Debug, Clone, PartialEq, Eq, TypedBuilder, Default, Serialize, Deserialize)]
pub struct TorrentMeta {
    /// The url of the torrent file
    pub url: String,
    #[builder(default)]
    content_len: Option<u64>,
    #[builder(default)]
    pub_date: Option<String>,
    #[builder(default)]
    save_path: Option<String>,
    #[builder(default)]
    category: Option<String>,
}

#[allow(dead_code)]
impl TorrentMeta {
    async fn download_dot_torrent(&self) -> Result<Vec<u8>, TorrentInaccessibleError> {
        let url = &self.url;
        let response = reqwest::get(url)
            .await
            .map_err(|e| TorrentInaccessibleError(url.to_string(), e.to_string()))?;
        let content = response
            .bytes()
            .await
            .map_err(|e| TorrentInaccessibleError(url.to_string(), e.to_string()))?;
        Ok(content.to_vec())
    }

    async fn fetch_torrent(&self) -> Result<Torrent, TorrentInaccessibleError> {
        let dot_torrent = self.download_dot_torrent().await?;
        Torrent::from_bytes(&dot_torrent)
            .map_err(|e| TorrentInaccessibleError(self.url.clone(), e.to_string()))
    }

    // Return a clone of the torrent in cache.
    // If it's not present, download the torrent and update the cache.
    async fn get_data(&self) -> Result<Torrent, TorrentInaccessibleError> {
        let mut cache_lock = TORRENT_CACHE.lock().await;
        let cache = cache_lock.get(&self.url);
        match cache {
            Some(torrent) => Ok(torrent.clone()),
            None => {
                let torrent = self.fetch_torrent().await?;
                cache_lock.put(self.url.clone(), torrent.clone());
                Ok(torrent)
            }
        }
    }

    pub async fn get_torrent_id(&self) -> Result<String, TorrentInaccessibleError> {
        let torrent = self.get_data().await?;
        Ok(hex::encode(&torrent.torrent_id()))
    }

    pub async fn get_name(&self) -> Result<String, TorrentInaccessibleError> {
        let torrent = self.get_data().await?;
        Ok(torrent.get_info_name())
    }
}

type TorrentCache = LruCache<String, Torrent>;
static TORRENT_CACHE: Lazy<Mutex<TorrentCache>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));

#[derive(Debug, thiserror::Error)]
#[error("Torrent inaccessible: {0}\n {1}")]
pub struct TorrentInaccessibleError(String, String);

#[derive(Debug, thiserror::Error)]
pub enum DownloaderError {
    #[error("Invalid authentication for downloader: {0}")]
    InvalidAuthentication(String),

    #[error("Downloader error: {0}")]
    ClientError(String),

    #[error("Torrent inaccessible: {0}")]
    TorrentInaccessibleError(#[from] TorrentInaccessibleError),

    #[error("Database error: {0}")]
    DBError(#[from] DBError),
}

#[derive(Debug, Clone)]
pub struct DownloadingTorrent {
    pub hash: String,
    pub status: TaskStatus,
    // Path where this torrent's data is stored
    pub save_path: String,
    // Torrent name
    // if the torrent is a single file, this is the file name, otherwise the directory name
    pub name: String,
}

impl DownloadingTorrent {
    pub fn get_file_path(&self) -> PathBuf {
        Path::new(&self.save_path).join(&self.name)
    }
}

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download(&self, torrent: &TorrentMeta) -> Result<(), DownloaderError>;

    async fn get_download_list(&self) -> Result<Vec<DownloadingTorrent>, DownloaderError>;

    async fn rename_file(
        &self,
        torrent: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> Result<(), DownloaderError>;
}

#[derive(Debug, Clone, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum DownloaderType {
    QBittorrent,
    Dummy,
}

pub async fn is_task_exist(torrent_url: &str) -> DBResult<bool> {
    let mut tx = tx_begin().await?;
    let exist = store::is_task_exist(&mut tx, torrent_url).await?;
    tx.rollback().await?;
    Ok(exist)
}

pub async fn download_with_state(
    downloader: Arc<Mutex<Box<dyn Downloader>>>,
    torrent_meta: &TorrentMeta,
    bangumi_info: &BangumiInfo,
) -> Result<(), DownloaderError> {
    let info_hash = torrent_meta.get_torrent_id().await?;

    let mut tx = tx_begin().await?;

    let created = store::add_task(
        &mut tx,
        &DownloadTask::builder()
            .id(None)
            .torrent_hash(info_hash)
            .torrent_url(Some(torrent_meta.url.to_string()))
            .status(TaskStatus::Downloading)
            .start_time(chrono::Local::now())
            .renamed(false)
            .build(),
        bangumi_info,
    )
    .await?;

    if created == 0 {
        // Skip downloading if the task is already created
        return Ok(());
    }

    let downloader_lock = downloader.lock().await;
    match downloader_lock.download(torrent_meta).await {
        Ok(_) => {
            tx.commit().await?;
        }
        Err(err) => {
            error!("Failed to download torrent: {}", err);
            tx.rollback().await?;
        }
    }

    Ok(())
}

pub async fn get_bangumi_info(torrent_hash: &str) -> DBResult<Option<BangumiInfo>> {
    let mut tx = tx_begin().await?;
    let info = store::get_bangumi_info(&mut tx, torrent_hash).await?;
    Ok(info)
}

pub async fn update_task_status(download_list: &Vec<DownloadingTorrent>) -> DBResult<()> {
    let mut tx = tx_begin().await?;

    for torrent in download_list {
        match store::get_task(&mut tx, &torrent.hash).await? {
            Some(task) => {
                if task.status != torrent.status {
                    store::update_task_status(
                        &mut tx,
                        &torrent.hash,
                        torrent.status,
                        torrent.get_file_path().as_path(),
                    )
                    .await?;
                }
            }
            None => {
                debug!("Skip updating task status created by other process: [{}]", &torrent.hash);
            }
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn set_task_renamed(torrent_hash: &str) -> DBResult<()> {
    let mut tx = tx_begin().await?;
    store::update_task_renamed(&mut tx, torrent_hash).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn is_renamed(torrent_hash: &str) -> DBResult<bool> {
    let mut tx = tx_begin().await?;
    let renamed = store::is_renamed(&mut tx, torrent_hash).await?;
    tx.rollback().await?;
    Ok(renamed)
}

#[allow(dead_code)]
static GLOBAL_DOWNLOADER: OnceCell<Arc<Mutex<Box<dyn Downloader>>>> = OnceCell::new();

pub fn get_downloader() -> Arc<Mutex<Box<dyn Downloader>>> {
    #[cfg(test)]
    {
        Arc::new(Mutex::new(Box::new(DummyDownloader::new())))
    }

    #[cfg(not(test))]
    match get_downloader_type() {
        Some(downloader_type) => {
            let downloader = GLOBAL_DOWNLOADER.get_or_init(|| {
                let downloader = init_downloader(downloader_type);
                Arc::new(Mutex::new(downloader))
            });

            downloader.clone()
        }
        None => panic!("Downloader type not set"),
    }
}

pub fn get_downloader_type() -> Option<DownloaderType> {
    match std::env::var("DOWNLOADER_TYPE") {
        Ok(downloader_type) => Some(
            DownloaderType::from_str(&downloader_type)
                .expect(&format!("Invalid downloader type, {}", &downloader_type)),
        ),
        Err(_) => None,
    }
}

pub fn init_downloader(downloader_type: DownloaderType) -> Box<dyn Downloader> {
    match downloader_type {
        DownloaderType::QBittorrent => {
            let username = std::env::var("DOWNLOADER_USERNAME").unwrap();
            let password = std::env::var("DOWNLOADER_PASSWORD").unwrap();
            let url = std::env::var("DOWNLOADER_HOST").unwrap();
            Box::new(QBittorrentDownloader::new(&username, &password, &url))
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
#[allow(unused)]
pub async fn update_torrent_cache(url: &str, torrent: &Torrent) {
    let mut cache_lock = TORRENT_CACHE.lock().await;
    cache_lock.put(url.to_string(), torrent.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_renamed() {
        use crate::{init, test::get_dummy_torrent};

        init().await;

        let downloader = get_downloader();
        let torrent = get_dummy_torrent().await;
        download_with_state(
            downloader.clone(),
            &torrent,
            &BangumiInfo::builder()
                .show_name("dummy".to_string())
                .episode_name(Some("".to_string()))
                .display_name(Some("".to_string()))
                .season(1u64)
                .episode(1u64)
                .category(None)
                .build(),
        )
        .await
        .unwrap();

        let torrent_hash = torrent.get_torrent_id().await.unwrap();
        assert_eq!(is_renamed(&torrent_hash).await.unwrap(), false);

        set_task_renamed(&torrent_hash).await.unwrap();
        assert_eq!(is_renamed(&torrent_hash).await.unwrap(), true);
    }
}
