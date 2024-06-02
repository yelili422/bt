use std::str::FromStr;

use super::{Downloader, DownloaderError, DownloadingTorrent, TaskStatus, TorrentMeta};
use async_trait::async_trait;
use qbit_rs::model::{Credential, State};

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
        let qtorrent = qbit_rs::model::AddTorrentArg::builder()
            .source(urls)
            .savepath(
                torrent
                    .save_path
                    .clone()
                    .unwrap_or(String::from("/downloads")),
            )
            .category(torrent.category.clone().unwrap_or(String::from("Bangumi")))
            .build();

        self.api
            .add_torrent(qtorrent)
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
        let torrent = crate::downloader::TorrentMetaBuilder::default()
            .url("https://mikanani.me/Download/20240111/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent")
            .content_len(1024u64)
            .pub_date("2021-01-01")
            .save_path("/downloads")
            .category("Bangumi")
            .build()
            .unwrap();

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
