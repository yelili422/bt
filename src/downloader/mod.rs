mod bittorrent;
mod bittorrent_meta;
mod downloaders;
pub mod store;
mod task;

use async_trait::async_trait;
use log::{debug, error, info};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use strum_macros::EnumString;
use tokio::sync::{oneshot, Mutex, OnceCell};
use tokio::task::JoinHandle;

use crate::renamer::BangumiInfo;
use crate::DBError;

pub use bittorrent::*;
pub use bittorrent_meta::*;
pub use task::*;

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

type DownloadingHook = Arc<dyn Fn(TaskStatus, &DownloadingTorrent) + Send + Sync>;

pub struct DownloadManager {
    // inner downloader
    downloader: Arc<Mutex<Box<dyn Downloader>>>,

    // use to refresh the downloading status and execute hook tasks
    shutdown_sender: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,

    hooks: Vec<DownloadingHook>,
}

impl DownloadManager {
    pub async fn new() -> Self {
        let downloader = Self::get_downloader().await;
        Self {
            downloader,
            shutdown_sender: None,
            handle: None,
            hooks: vec![],
        }
    }

    pub fn add_hook<F>(&mut self, hook: F)
    where
        F: Fn(TaskStatus, &DownloadingTorrent) + Send + Sync + 'static,
    {
        self.hooks.push(Arc::new(hook));
    }

    pub fn start(&mut self) {
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let hooks = self.hooks.clone();
        let downloader = self.downloader.clone();
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = async {
                    loop {
                        _ = Self::update_downloading_status(downloader.clone(), &hooks).await;
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                } => {},
                _ = shutdown_receiver => {
                    info!("[downloader] Shutdown signal received.");
                },
            }
            info!("[downloader] Shuting down...");
        });

        self.handle = Some(handle);
        self.shutdown_sender = Some(shutdown_sender);
    }

    pub async fn download_with_state(
        &self,
        rss_id: Option<i64>,
        torrent_meta: &TorrentMeta,
        bangumi_info: &BangumiInfo,
    ) -> Result<(), DownloaderError> {
        if store::is_task_exist(&torrent_meta.url).await? {
            // Skip downloading if the torrent info already in the database .
            return Ok(());
        }

        let mut torrent_meta = torrent_meta.clone();

        // Set default torrent save path and category
        if torrent_meta.save_path.is_none() {
            torrent_meta.save_path = Some("/downloads/bangumi".to_string());
        }
        if torrent_meta.category.is_none() {
            torrent_meta.category = Some("Bangumi".to_string());
        }

        let downloader_lock = self.downloader.lock().await;
        {
            match downloader_lock.download(&torrent_meta).await {
                Ok(_) => {
                    let info_hash = torrent_meta.get_torrent_id().await?;
                    let task = DownloadTask::builder()
                        .id(None)
                        .rss_id(rss_id)
                        .torrent_hash(info_hash)
                        .torrent_url(Some(torrent_meta.url.to_string()))
                        .status(TaskStatus::Downloading)
                        .start_time(chrono::Local::now())
                        .renamed(false)
                        .build();

                    store::add_task(rss_id, &task, bangumi_info).await?;
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }
    }

    async fn get_downloader() -> Arc<Mutex<Box<dyn Downloader>>> {
        #[cfg(test)]
        {
            Arc::new(Mutex::new(Box::new(downloaders::DummyDownloader::new())))
        }

        #[cfg(not(test))]
        match Self::get_downloader_type() {
            Some(downloader_type) => {
                let downloader = GLOBAL_DOWNLOADER
                    .get_or_init(|| async {
                        let downloader = Self::init_downloader(downloader_type);
                        Arc::new(Mutex::new(downloader))
                    })
                    .await;

                downloader.clone()
            }
            None => panic!("Downloader type not set"),
        }
    }

    #[allow(unused)]
    fn get_downloader_type() -> Option<DownloaderType> {
        match std::env::var("DOWNLOADER_TYPE") {
            Ok(downloader_type) => Some(
                DownloaderType::from_str(&downloader_type)
                    .expect(&format!("Invalid downloader type, {}", &downloader_type)),
            ),
            Err(_) => None,
        }
    }

    #[allow(unused)]
    fn init_downloader(downloader_type: DownloaderType) -> Box<dyn Downloader> {
        match downloader_type {
            DownloaderType::QBittorrent => {
                let username = std::env::var("DOWNLOADER_USERNAME").unwrap();
                let password = std::env::var("DOWNLOADER_PASSWORD").unwrap();
                let url = std::env::var("DOWNLOADER_HOST").unwrap();
                Box::new(downloaders::QBittorrentDownloader::new(&username, &password, &url))
            }
            _ => unreachable!(),
        }
    }

    async fn update_downloading_status(
        downloader: Arc<Mutex<Box<dyn Downloader>>>,
        hooks: &Vec<DownloadingHook>,
    ) -> Result<(), DownloaderError> {
        let downloader_lock = downloader.lock().await;
        {
            let download_list = downloader_lock.get_download_list().await?;
            for torrent in download_list {
                match store::get_task(&torrent.hash).await? {
                    Some(task_in_store) => {
                        if task_in_store.status != torrent.status {
                            store::update_task_status(
                                &torrent.hash,
                                torrent.status,
                                torrent.get_file_path().as_path(),
                            )
                            .await?;

                            for hook in hooks {
                                hook(torrent.status, &torrent);
                            }
                        }
                    }
                    None => {
                        debug!(
                            "Skip updating task status created by other process: [{}]",
                            &torrent.hash
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.send(());
        }

        // Wait for the task to finish
        if let Some(handle) = self.handle.take() {
            tokio::spawn(async move {
                let _ = handle.await;
            });
        }
    }
}

#[allow(dead_code)]
static GLOBAL_DOWNLOADER: OnceCell<Arc<Mutex<Box<dyn Downloader>>>> = OnceCell::const_new();

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_renamed() {
        use crate::{init, test::get_dummy_torrent};

        init().await;

        let downloader = DownloadManager::new().await;
        let torrent = get_dummy_torrent().await;

        downloader
            .download_with_state(
                None,
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
        assert_eq!(store::is_renamed(&torrent_hash).await.unwrap(), false);

        store::update_task_renamed(&torrent_hash).await.unwrap();
        assert_eq!(store::is_renamed(&torrent_hash).await.unwrap(), true);
    }
}
