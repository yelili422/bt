mod bittorrent;
mod dummy;
mod qbittorrent;
mod store;
mod task;

use async_trait::async_trait;
use derive_builder::Builder;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strum_macros::EnumString;
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

use crate::renamer::BangumiInfo;
use crate::tx_begin;
pub use bittorrent::*;
pub use dummy::DummyDownloader;
pub use qbittorrent::QBittorrentDownloader;
pub use task::*;

/// The metadata of a torrent file
#[derive(Debug, Clone, Builder, Default, Serialize, Deserialize)]
#[builder(setter(into, strip_option), default)]
pub struct TorrentMeta {
    /// The url of the torrent file
    url: String,
    #[serde(skip_serializing, skip_deserializing)]
    data: Arc<Mutex<Option<Torrent>>>,
    content_len: Option<u64>,
    pub_date: Option<String>,
    save_path: Option<String>,
    category: Option<String>,
}

impl PartialEq<Self> for TorrentMeta {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.content_len == other.content_len
            && self.pub_date == other.pub_date
            && self.save_path == other.save_path
            && self.category == other.category
    }
}

impl Eq for TorrentMeta {}

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

    async fn fetch_torrent(&self) -> Result<(), TorrentInaccessibleError> {
        let mut data_lock = self.data.lock().await;
        {
            if data_lock.is_none() {
                let dot_torrent = self.download_dot_torrent().await?;
                let torrent = Torrent::from_bytes(&dot_torrent)
                    .map_err(|e| TorrentInaccessibleError(self.url.clone(), e.to_string()))?;
                *data_lock = Some(torrent);
            }
        }

        Ok(())
    }

    async fn get_data(&self) -> MutexGuard<Option<Torrent>> {
        self.data.lock().await
    }

    async fn get_info_hash(&self) -> Result<String, TorrentInaccessibleError> {
        let data_lock = self.get_data().await;
        match &*data_lock {
            Some(torrent) => Ok(hex::encode(torrent.info_hash())),
            None => {
                panic!("Torrent data not fetched")
            }
        }
    }

    async fn get_name(&self) -> Result<String, TorrentInaccessibleError> {
        match &*self.get_data().await {
            Some(torrent) => Ok(torrent.info.name.clone()),
            None => {
                panic!("Torrent data not fetched")
            }
        }
    }
}

#[derive(Debug, Error)]
#[error("Torrent inaccessible: {0}\n {1}")]
pub struct TorrentInaccessibleError(String, String);

#[derive(Debug, Error)]
pub enum DownloaderError {
    #[error("Invalid authentication for downloader: {0}")]
    InvalidAuthentication(String),

    #[error("Downloader error: {0}")]
    ClientError(String),

    #[error("Torrent inaccessible: {0}")]
    TorrentInaccessibleError(#[from] TorrentInaccessibleError),
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
}

#[derive(Debug, Clone, EnumString)]
#[strum(serialize_all = "lowercase")]
enum DownloaderType {
    QBittorrent,
    Dummy,
}

pub async fn download_with_state(
    downloader: Arc<Mutex<Box<dyn Downloader>>>,
    torrent_meta: &TorrentMeta,
    bangumi_info: &BangumiInfo,
) -> anyhow::Result<()> {
    torrent_meta.fetch_torrent().await?;
    let info_hash = torrent_meta.get_info_hash().await?;

    let mut tx = tx_begin().await?;

    let created = store::add_task(
        &mut tx,
        &DownloadTaskBuilder::default()
            .id(None)
            .torrent_hash(info_hash)
            .torrent_url(Some(torrent_meta.url.to_string()))
            .status(TaskStatus::Downloading)
            .start_time(chrono::Local::now())
            .renamed(false)
            .build()
            .unwrap(),
        bangumi_info,
    )
    .await?;

    // FIXME: if download task is not created in downloader, rollback
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

pub async fn get_bangumi_info(torrent_hash: &str) -> anyhow::Result<Option<BangumiInfo>> {
    let mut tx = tx_begin().await?;
    let info = store::get_bangumi_info(&mut tx, torrent_hash).await?;
    Ok(info)
}

pub async fn update_task_status(download_list: &Vec<DownloadingTorrent>) -> anyhow::Result<()> {
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

pub async fn set_task_renamed(torrent_hash: &str) -> anyhow::Result<()> {
    let mut tx = tx_begin().await?;
    store::update_task_renamed(&mut tx, torrent_hash).await?;
    tx.rollback().await?;
    Ok(())
}

#[cfg(not(test))]
pub fn get_downloader() -> Box<dyn Downloader> {
    use std::env;
    use std::str::FromStr;

    let downloader_type = env::var("DOWNLOADER_TYPE").unwrap_or_default();
    match DownloaderType::from_str(&downloader_type) {
        Ok(DownloaderType::QBittorrent) => {
            let username = env::var("DOWNLOADER_USERNAME").unwrap_or_default();
            let password = env::var("DOWNLOADER_PASSWORD").unwrap_or_default();
            let url = env::var("DOWNLOADER_HOST").unwrap_or_default();
            Box::new(QBittorrentDownloader::new(&username, &password, &url))
        }
        _ => panic!("Invalid downloader type, {}", downloader_type),
    }
}

#[cfg(test)]
pub fn get_downloader() -> Box<dyn Downloader> {
    Box::new(DummyDownloader::new())
}
