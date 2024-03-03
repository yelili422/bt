pub mod parsers;
mod store;

use crate::downloader::Torrent;
use crate::rss::parsers::RssParser;

#[derive(Debug, PartialEq, Eq)]
pub struct Rss<'a, Parser> {
    parser: &'a Parser,
    pub url: String,
    pub title: Option<String>,
}

impl<'a, Parser> Rss<'a, Parser>
where
    Parser: RssParser,
{
    pub fn new(url: String, title: Option<String>, parser: &'a Parser) -> Self {
        Rss { parser, url, title }
    }
}

/// The rss subscription content struct
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
