#[derive(Debug, PartialEq, Eq)]
pub struct Torrent {
    pub url: String,
    pub content_len: u64,
    pub pub_date: String,
}

impl Torrent {
    pub fn new(url: String, content_len: u64, pub_date: String) -> Self {
        Self {
            url,
            content_len,
            pub_date,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DownloadError(String);

impl std::ops::Deref for DownloadError {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait Downloader {
    async fn download(&self, url: &str) -> Result<String, DownloadError>;
}
