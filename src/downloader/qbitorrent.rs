use super::{Downloader, DownloaderError, Torrent};

struct QBittorrentDownloader {
    username: String,
    password: String,
    address: String,
}

impl QBittorrentDownloader {
    fn new(username: &str, password: &str, address: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            address: address.to_string(),
        }
    }

    async fn get_session(&self) -> Result<qbittorrent::Api, DownloaderError> {
        // TODO: Reuse the session if it's still valid
        Ok(self.login().await?)
    }

    async fn login(&self) -> Result<qbittorrent::Api, DownloaderError> {
        let api = qbittorrent::Api::new(&self.username, &self.password, &self.address)
            .await
            .or_else(|err| Err(DownloaderError::InvalidAuthentication(err.to_string())))?;

        Ok(api)
    }

    async fn application_version(&self) -> Result<String, DownloaderError> {
        self.get_session()
            .await?
            .application_version()
            .await
            .map_err(|err| DownloaderError::UnknownError(err.to_string()))
    }

    async fn get_torrent_list(&self) -> Result<Vec<qbittorrent::data::Torrent>, DownloaderError> {
        self.get_session()
            .await?
            .get_torrent_list()
            .await
            .map_err(|err| DownloaderError::UnknownError(err.to_string()))
    }
}

impl Downloader for QBittorrentDownloader {
    async fn download(&self, torrent: Torrent) -> Result<(), DownloaderError> {
        let qtorrent = qbittorrent::queries::TorrentDownloadBuilder::default()
            .urls(torrent.url.expect("Empty torrent URL is not allowed"))
            .savepath(torrent.save_path.unwrap_or("/downloads".to_string()))
            .category(torrent.category.unwrap_or("Bangumi".to_string()))
            .build()
            .map_err(|err| DownloaderError::UnknownError(err.to_string()))?;

        self.get_session()
            .await?
            .add_new_torrent(&qtorrent)
            .await
            .map_err(|err| DownloaderError::UnknownError(err.to_string()))
    }
}

#[allow(unused_imports)]
mod tests {
    use core::time;

    use crate::downloader::{Downloader, DownloaderError};

    use super::QBittorrentDownloader;

    async fn get_downloader() -> Result<QBittorrentDownloader, DownloaderError> {
        Ok(QBittorrentDownloader::new(
            "admin",
            "adminadmin",
            "http://localhost:8080",
        ))
    }

    #[tokio::test]
    async fn login() {
        let downloader = get_downloader().await.unwrap();
        let version = downloader.application_version().await.unwrap();
        assert_ne!(version, "Forbidden");
        assert_ne!(version, "");
    }

    #[tokio::test]
    async fn download() {
        let downloader = get_downloader().await.unwrap();
        let torrent = crate::downloader::TorrentBuilder::default()
            .url("https://mikanani.me/Download/20240111/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent")
            .content_len(1024u64)
            .pub_date("2021-01-01")
            .save_path("/downloads")
            .category("Bangumi")
            .build()
            .unwrap();

        let result = downloader.download(torrent).await;
        assert!(result.is_ok());

        tokio::time::sleep(time::Duration::from_secs(2)).await;

        let torrents = downloader.get_torrent_list().await.unwrap();
        assert!(torrents.len() >= 1);
    }
}
