mod parsers;

use crate::downloader::Torrent;

#[derive(Debug, PartialEq, Eq)]
pub struct RssSubscription {
    pub url: String,
    pub description: String,
    pub items: Vec<RssSubscriptionItem>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RssSubscriptionItem {
    pub url: String,
    pub title: String,
    pub episode_title: String,
    pub description: String,
    pub season: u64,
    pub episode: u64,
    pub fansub: String,
    pub media_info: String,
    pub torrent: Torrent,
}
