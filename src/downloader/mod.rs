mod bittorrent;
mod qbittorrent;
mod store;
mod task;

use async_trait::async_trait;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::get_pool;
use crate::renamer::BangumiInfo;
pub use bittorrent::*;
pub use qbittorrent::QBittorrentDownloader;
pub use task::*;

/// The metadata of a torrent file
#[derive(Debug, Clone, PartialEq, Eq, Builder, Default, Serialize, Deserialize)]
#[builder(setter(into, strip_option), default)]
pub struct TorrentMeta {
    /// The url of the torrent file
    url: String,
    content_len: Option<u64>,
    pub_date: Option<String>,
    save_path: Option<String>,
    category: Option<String>,
}

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

#[derive(Debug)]
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

pub async fn download_with_state(
    downloader: &dyn Downloader,
    torrent_meta: &TorrentMeta,
    bangumi_info: &BangumiInfo,
) -> anyhow::Result<()> {
    let dot_torrent = torrent_meta.download_dot_torrent().await?;

    let torrent: Torrent = serde_bencode::from_bytes(&dot_torrent).unwrap();
    let info_hash = hex::encode(torrent.info_hash());

    store::add_task(
        &get_pool().await?,
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

    downloader.download(torrent_meta).await?;

    Ok(())
}

pub async fn get_bangumi_info(torrent_hash: &str) -> anyhow::Result<BangumiInfo> {
    let info = store::get_bangumi_info(&get_pool().await?, torrent_hash).await?;

    Ok(info)
}

pub async fn update_task_status(download_list: &Vec<DownloadingTorrent>) -> anyhow::Result<()> {
    let pool = get_pool().await?;
    for torrent in download_list {
        let task = store::get_task(&pool, &torrent.hash).await?;
        if task.status != torrent.status {
            store::update_task_status(
                &pool,
                &torrent.hash,
                torrent.status,
                torrent.get_file_path().as_path(),
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn set_task_renamed(torrent_hash: &str) -> anyhow::Result<()> {
    let pool = get_pool().await?;
    store::update_task_renamed(&pool, torrent_hash).await?;
    Ok(())
}

pub fn get_downloader() -> Box<dyn Downloader> {
    // TODO: Read from config
    Box::new(QBittorrentDownloader::new("admin", "adminadmin", "http://localhost:8080"))
}
