use std::str::FromStr;

use async_trait::async_trait;
use qbit_rs::model::{Credential, State};

use crate::downloader::{
    bittorrent_meta::TorrentMeta, Downloader, DownloaderError, DownloadingTorrent, TaskStatus,
};

pub struct QBittorrentDownloader {
    api: qbit_rs::Qbit,
}

#[allow(unused)]
impl QBittorrentDownloader {
    pub fn new(username: &str, password: &str, address: &str) -> Self {
        let credential = Credential::new(username, password);
        let api = qbit_rs::Qbit::new(address, credential);

        Self { api }
    }

    async fn application_version(&self) -> Result<String, DownloaderError> {
        self.api
            .get_version()
            .await
            .map_err(|err| DownloaderError::ClientError(err.to_string()))
    }

    async fn get_torrent_list(&self) -> Result<Vec<qbit_rs::model::Torrent>, DownloaderError> {
        let params = qbit_rs::model::GetTorrentListArg::default();
        self.api
            .get_torrent_list(params)
            .await
            .map_err(|err| DownloaderError::ClientError(err.to_string()))
    }
}

#[async_trait]
impl Downloader for QBittorrentDownloader {
    async fn download(&self, torrent: &TorrentMeta) -> Result<(), DownloaderError> {
        let urls = qbit_rs::model::TorrentSource::Urls {
            urls: qbit_rs::model::Sep::from_str(&torrent.url).unwrap(),
        };

        let task = qbit_rs::model::AddTorrentArg {
            source: urls,
            savepath: torrent.save_path.clone(),
            cookie: None,
            category: torrent.category.clone(),
            tags: None,
            skip_checking: None,
            paused: None,
            root_folder: None,
            rename: None,
            up_limit: None,
            download_limit: None,
            ratio_limit: None,
            seeding_time_limit: None,
            auto_torrent_management: None,
            sequential_download: None,
            first_last_piece_priority: None,
        };

        self.api
            .add_torrent(task)
            .await
            .map_err(|err| DownloaderError::ClientError(err.to_string()))
    }

    async fn get_download_list(&self) -> Result<Vec<DownloadingTorrent>, DownloaderError> {
        let download_tasks = self
            .get_torrent_list()
            .await?
            .iter()
            .map(|t| {
                let status = match t.state.clone().unwrap_or(State::Unknown) {
                    State::PausedDL => TaskStatus::Pause,
                    State::Uploading | State::PausedUP | State::QueuedUP | State::StalledUP => {
                        TaskStatus::Completed
                    }
                    State::Allocating
                    | State::CheckingUP
                    | State::ForcedUP
                    | State::Downloading
                    | State::QueuedDL
                    | State::StalledDL
                    | State::CheckingDL
                    | State::ForcedDL
                    | State::CheckingResumeData
                    | State::MetaDL => TaskStatus::Downloading,
                    State::Error | State::MissingFiles | State::Unknown | State::Moving => {
                        TaskStatus::Error
                    }
                };

                DownloadingTorrent {
                    hash: t.hash.clone().unwrap_or_default(),
                    status,
                    save_path: t.save_path.clone().unwrap_or_default(),
                    name: t.name.clone().unwrap_or_default(),
                }
            })
            .collect();

        Ok(download_tasks)
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
#[allow(unused_imports, unused)]
mod tests {
    use core::time;

    use crate::downloader::{Downloader, DownloaderError};

    use super::QBittorrentDownloader;

    fn get_downloader() -> Result<QBittorrentDownloader, DownloaderError> {
        Ok(QBittorrentDownloader::new("admin", "adminadmin", "http://localhost:8080"))
    }

    #[ignore]
    #[tokio::test]
    async fn login() {
        let downloader = get_downloader().unwrap();
        let version = downloader.application_version().await.unwrap();
        assert_ne!(version, "Forbidden");
        assert_ne!(version, "");
    }

    #[ignore]
    #[tokio::test]
    async fn download() {
        let downloader = get_downloader().unwrap();
        let torrent = crate::downloader::TorrentMeta::builder()
            .url("https://mikanani.me/Download/20240111/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent".to_string())
            .save_path(Some("/downloads".to_string()))
            .category(Some("Bangumi".to_string()))
            .build();

        downloader.download(&torrent).await.unwrap();

        tokio::time::sleep(time::Duration::from_secs(2)).await;

        let torrents = downloader.get_torrent_list().await.unwrap();
        dbg!(&torrents);
        assert!(torrents.len() >= 1);
    }

    #[ignore]
    #[tokio::test]
    async fn get_download_list() {
        let downloader = get_downloader().unwrap();
        let torrents = downloader.get_download_list().await.unwrap();
        dbg!(&torrents);
    }
}
