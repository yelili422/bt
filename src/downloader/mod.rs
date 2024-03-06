mod qbitorrent;
mod store;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

/// The metadata of a torrent file
#[derive(Debug, PartialEq, Eq, Builder, Default, Serialize, Deserialize)]
#[builder(setter(into, strip_option), default)]
pub struct TorrentMeta {
    /// The url of the torrent file
    url: Option<String>,
    /// The content of the torrent file
    data: Option<Vec<u8>>,
    content_len: Option<u64>,
    pub_date: Option<String>,
    save_path: Option<String>,
    category: Option<String>,
}

impl TorrentMeta {
    async fn download_content(&self) -> anyhow::Result<Vec<u8>> {
        if let Some(url) = &self.url {
            let response = reqwest::get(url).await?;
            let content = response.bytes().await?;
            Ok(content.to_vec())
        } else {
            panic!("Empty torrent URL cannot be downloaded")
        }
    }
}

#[derive(Debug)]
pub enum DownloaderError {
    InvalidAuthentication(String),
    UnknownError(String),
}

pub trait Downloader {
    async fn download(&self, torrent: TorrentMeta) -> Result<(), DownloaderError>;
}

pub async fn download_with_state<T: Downloader>(
    downloader: T,
    torrents: Vec<TorrentMeta>,
) -> Result<(), DownloaderError> {
    for torrent in torrents {
        downloader.download(torrent).await?;
    }

    Ok(())
}
