mod dummy;
mod qbittorrent;

#[cfg(test)]
pub use dummy::DummyDownloader;
pub use qbittorrent::QBittorrentDownloader;
