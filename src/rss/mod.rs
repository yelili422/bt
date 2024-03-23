pub mod parsers;
pub mod store;

use crate::downloader::TorrentMeta;
use crate::renamer::{BangumiInfo, BangumiInfoBuilder};
use crate::rss::store::RssEntity;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Builder, PartialEq, Eq)]
#[builder(setter(into))]
pub struct Rss {
    pub url: String,
    pub title: Option<String>,
    pub rss_type: RssType,
    pub season: Option<u64>,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RssType {
    Mikan,
}

impl From<RssEntity> for Rss {
    fn from(entity: RssEntity) -> Self {
        Rss {
            url: entity.url,
            title: entity.title,
            rss_type: entity.rss_type,
            season: entity.season,
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
            // Display name is necessary because some bangumies have multiple versions
            // from different platforms
            // When renaming the file, the display name is used as the file name to avoid conflicts
            .display_name(s.media_info.clone())
            .season(s.season)
            .episode(s.episode)
            .category(None)
            .build()
            .unwrap()
    }
}
