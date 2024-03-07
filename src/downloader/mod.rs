mod bittorrent;
mod qbittorrent;
mod store;

use crate::downloader::bittorrent::Torrent;
use async_trait::async_trait;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::downloader::store::DownloadTaskBuilder;
use crate::get_pool;
use crate::renamer::BangumiInfo;
pub use qbittorrent::QBittorrentDownloader;

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

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download(&self, torrent: &TorrentMeta) -> Result<(), DownloaderError>;
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
            .status(store::TaskStatus::Downloading)
            .start_time(chrono::Local::now())
            .end_time(None)
            .build()
            .unwrap(),
        bangumi_info,
    )
    .await?;

    downloader.download(torrent_meta).await?;

    Ok(())
}

pub fn get_downloader() -> Box<dyn Downloader> {
    // TODO: Read from config
    Box::new(QBittorrentDownloader::new("admin", "adminadmin", "http://localhost:8080"))
}
