use core::panic;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use lru::LruCache;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use typed_builder::TypedBuilder;

use super::{TaskStatus, Torrent, TorrentInaccessibleError};

/// The metadata of a torrent file
#[derive(Debug, Clone, PartialEq, Eq, TypedBuilder, Default, Serialize, Deserialize)]
pub struct TorrentMeta {
    /// The url of the torrent file
    pub url: String,
    /// Download path
    #[builder(default)]
    pub save_path: Option<String>,
    #[builder(default)]
    pub category: Option<String>,
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

#[cfg(test)]
#[allow(unused)]
pub async fn update_torrent_cache(url: &str, torrent: &Torrent) {
    let mut cache_lock = TORRENT_CACHE.lock().await;
    cache_lock.put(url.to_string(), torrent.clone());
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
