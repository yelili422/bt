pub mod parsers;
pub mod store;

use crate::downloader::TorrentMeta;
use crate::renamer::{BangumiInfo, BangumiInfoBuilder};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, PartialEq, Eq)]
pub struct Rss {
    pub url: String,
    pub title: Option<String>,
    pub rss_type: RssType,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum RssType {
    Mikan,
}

impl Rss {
    pub fn new(url: String, title: Option<String>, rss_type: RssType) -> Self {
        Rss {
            url,
            title,
            rss_type,
        }
    }
}

/// The rss subscription content struct
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RssSubscription {
    pub url: String,
    pub description: String,
    pub items: Vec<RssSubscriptionItem>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RssSubscriptionItem {
    pub url: String,
    pub title: String,
    pub episode_title: String,
    pub description: String,
    pub season: u64,
    pub episode: u64,
    pub fansub: String,
    pub media_info: String,
    pub torrent: TorrentMeta,
}

impl From<&RssSubscriptionItem> for BangumiInfo {
    fn from(s: &RssSubscriptionItem) -> Self {
        BangumiInfoBuilder::default()
            .show_name(s.title.clone())
            .episode_name(s.episode_title.clone())
            .display_name(s.media_info.clone())
            .season(s.season)
            .episode(s.episode)
            .category(None)
            .build()
            .unwrap()
    }
}
