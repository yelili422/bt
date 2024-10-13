use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use typed_builder::TypedBuilder;

use crate::downloader::TorrentMeta;
use crate::renamer::BangumiInfo;
use crate::rss::filter::RssFilterChain;

mod filter;
pub mod parsers;
pub mod store;

#[derive(Debug, Clone, TypedBuilder, Serialize, Deserialize)]
pub struct Rss {
    /// The primary key of the rss
    #[builder(default)]
    pub id: Option<i64>,
    /// The url of the rss
    pub url: String,
    /// The title of the show
    #[builder(default)]
    pub title: Option<String>,
    /// The type of the rss
    pub rss_type: RssType,
    /// The season of the show
    #[builder(default)]
    pub season: Option<u64>,
    #[builder(default)]
    pub enabled: Option<bool>,
    #[builder(default)]
    pub filters: Option<RssFilterChain>,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub category: Option<String>,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RssType {
    Mikan,
}

/// The rss subscription content struct
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RssSubscription {
    pub url: String,
    pub items: Vec<RssSubscriptionItem>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, TypedBuilder)]
pub struct RssSubscriptionItem {
    pub url: String,
    pub title: String,
    pub episode_title: String,
    pub season: u64,
    pub episode: u64,
    pub fansub: String,
    pub media_info: String,
    pub torrent: TorrentMeta,
    pub category: String,
}

impl From<&RssSubscriptionItem> for BangumiInfo {
    fn from(s: &RssSubscriptionItem) -> Self {
        let display_name = format!("{}{}", s.fansub, s.media_info);

        BangumiInfo::builder()
            .show_name(s.title.clone())
            .episode_name({
                if s.episode_title.is_empty() {
                    None
                } else {
                    Some(s.episode_title.clone())
                }
            })
            // Display name is necessary because some bangumies have multiple versions
            // from different platforms
            // When renaming the file, the display name is used as the file name to avoid conflicts
            .display_name({
                if display_name.is_empty() {
                    None
                } else {
                    Some(display_name)
                }
            })
            .season(s.season)
            .episode(s.episode)
            .category({
                if s.category.is_empty() {
                    None
                } else {
                    Some(s.category.clone())
                }
            })
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_item_to_bangumi_info() {
        let rss_item = RssSubscriptionItem {
            url: "https://mikanani.me/Home/Episode/059724511d60173251b378b04709aceff92fffb5".to_string(),
            title: "葬送的芙莉莲".to_string(),
            episode_title: "".to_string(),
            season: 1,
            episode: 18,
            fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
            media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
            category: "".to_string(),
            torrent: crate::downloader::TorrentMeta::builder()
                .url("https://mikanani.me/Download/20240118/059724511d60173251b378b04709aceff92fffb5.torrent".to_string())
                .build(),
        };

        let bangumi_info: BangumiInfo = (&rss_item).into();
        assert_eq!(bangumi_info.show_name, "葬送的芙莉莲");
        assert_eq!(bangumi_info.episode_name, None);
        assert_eq!(
            bangumi_info.display_name,
            Some(String::from(
                "[喵萌奶茶屋&LoliHouse][WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]"
            ))
        );
        assert_eq!(bangumi_info.season, 1);
        assert_eq!(bangumi_info.episode, 18);
    }
}
