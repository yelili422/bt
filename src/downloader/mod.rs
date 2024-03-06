mod bittorrent;
mod qbittorrent;
mod store;

use crate::downloader::bittorrent::Torrent;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The metadata of a torrent file
#[derive(Debug, PartialEq, Eq, Builder, Default, Serialize, Deserialize)]
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

pub trait Downloader {
    async fn download(&self, torrent: TorrentMeta) -> Result<(), DownloaderError>;
}

pub async fn download_with_state<T: Downloader>(
    downloader: T,
    torrent_meta: TorrentMeta,
) -> Result<(), DownloaderError> {
    let dot_torrent = torrent_meta.download_dot_torrent().await?;

    let torrent: Torrent = serde_bencode::from_bytes(&dot_torrent).unwrap();
    let info_hash = hex::encode(torrent.info_hash());

    dbg!(&info_hash);

    downloader.download(torrent_meta).await?;

    Ok(())
}
