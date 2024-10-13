use crate::downloader::{Downloader, DownloaderError, DownloadingTorrent, TaskStatus, TorrentMeta};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DummyDownloader {
    download_list: Arc<Mutex<Vec<DownloadingTorrent>>>,
}

impl DummyDownloader {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            download_list: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Downloader for DummyDownloader {
    async fn download(&self, torrent: &TorrentMeta) -> Result<(), DownloaderError> {
        let name = torrent.get_name().await.unwrap();
        let hash = torrent.get_torrent_id().await.unwrap();
        let category = torrent.category.as_deref().unwrap_or("bangumi");

        let path = format!("./data/dummy/downloads/{}/{}", category, name);
        let path = Path::new(&path);
        if !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"dummy").unwrap();
        }

        let downloading_torrent = DownloadingTorrent {
            hash,
            status: TaskStatus::Completed,
            save_path: path.display().to_string(),
            name,
        };

        let mut download_list_lock = self.download_list.lock().await;
        {
            download_list_lock.push(downloading_torrent);
        }
        Ok(())
    }

    async fn get_download_list(&self) -> Result<Vec<DownloadingTorrent>, DownloaderError> {
        let download_list = self.download_list.lock().await;
        Ok(download_list.clone())
    }

    async fn rename_file(
        &self,
        _torrent: &str,
        _old_path: &std::path::Path,
        _new_path: &std::path::Path,
    ) -> Result<(), DownloaderError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_downloader_works() {
        use crate::test::get_dummy_torrent;

        let downloader = DummyDownloader::new();
        let torrent = get_dummy_torrent().await;

        downloader.download(&torrent).await.unwrap();
        let download_list = downloader.get_download_list().await.unwrap();
        assert_eq!(download_list.len(), 1);

        let path = download_list[0].save_path.clone();
        assert!(Path::new(&path).exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "dummy");
    }
}
