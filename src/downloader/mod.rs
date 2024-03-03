mod qbitorrent;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::rss::RssSubscriptionItem;

#[derive(Debug, PartialEq, Eq, Builder, Default, Serialize, Deserialize)]
#[builder(setter(into, strip_option), default)]
pub struct Torrent {
    url: Option<String>,
    content_len: Option<u64>,
    pub_date: Option<String>,
    save_path: Option<String>,
    category: Option<String>,
}

#[derive(Debug)]
pub enum DownloaderError {
    InvalidAuthentication(String),
    UnknownError(String),
}

pub trait Downloader {
    async fn download(&self, torrent: Torrent) -> Result<(), DownloaderError>;
}

#[derive(Default, Builder, Debug, PartialEq, Eq)]
#[builder(setter(into))]
pub struct TvRules {
    pub show_name: String,
    pub episode_name: String,
    pub display_name: String,
    pub season: u64,
    pub episode: u64,
    pub category: String,
}

impl From<&RssSubscriptionItem> for TvRules {
    fn from(s: &RssSubscriptionItem) -> Self {
        TvRulesBuilder::default()
            .show_name(s.title.clone())
            .episode_name(s.episode_title.clone())
            .display_name(s.media_info.clone())
            .season(s.season)
            .episode(s.episode)
            .build()
            .unwrap()
    }
}
