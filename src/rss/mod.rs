use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use typed_builder::TypedBuilder;

use crate::downloader::TorrentMeta;
use crate::renamer::BangumiInfo;
use crate::rss::filter::RssFilterChain;
use crate::{tx_begin, DBError};

mod filter;
pub mod parsers;
mod store;

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

pub async fn list_rss() -> Result<Vec<Rss>, DBError> {
    let mut tx = tx_begin().await?;
    let rss_list = store::query_rss(&mut tx).await?;
    tx.rollback().await?;
    Ok(rss_list)
}

pub async fn add_rss(info: &Rss) -> Result<i64, DBError> {
    let mut tx = tx_begin().await?;

    let id = match store::check_repeat_by_url(&mut tx, &info.url).await? {
        Some(id) => {
            info!("[store] RSS url {} already exists", &info.url);
            id
        }
        None => store::insert_rss(&mut tx, &info).await?,
    };

    tx.commit().await?;
    Ok(id)
}

pub async fn delete_rss(id: i64) -> Result<(), DBError> {
    let mut tx = tx_begin().await?;
    store::delete_rss(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_rss(id: i64, info: &Rss) -> Result<(), DBError> {
    let mut tx = tx_begin().await?;
    store::update_rss(&mut tx, id, &info).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;

    #[tokio::test]
    async fn test_rss() {
        init().await;

        let rss_list = list_rss().await.unwrap();
        assert_eq!(rss_list.len(), 0);

        let mut rss = Rss::builder()
            .title(Some("Sousou no Frieren".to_string()))
            .url(
                "https://mikanani.me/Home/Episode/059724511d60173251b378b04709aceff92fffb5"
                    .to_string(),
            )
            .rss_type(RssType::Mikan)
            .season(Some(1))
            .enabled(Some(true))
            .build();

        let id = add_rss(&rss).await.unwrap();
        let rss_list = list_rss().await.unwrap();
        assert_eq!(rss_list.len(), 1);

        assert_eq!(rss_list[0].id, Some(id));
        assert_eq!(rss_list[0].url, rss.url);
        assert_eq!(rss_list[0].rss_type, rss.rss_type);
        assert_eq!(rss_list[0].season, rss.season);
        assert_eq!(rss_list[0].enabled, rss.enabled);
        assert_eq!(rss_list[0].title, rss.title);

        rss.title = Some("Frieren: Beyond Journey's End".to_string());
        assert_eq!(add_rss(&rss).await.unwrap(), id);

        update_rss(id, &rss).await.unwrap();

        let rss_list = list_rss().await.unwrap();
        assert_eq!(rss_list.len(), 1);
        assert_eq!(rss_list[0].title, rss.title);

        delete_rss(id).await.unwrap();
        let rss_list = list_rss().await.unwrap();
        assert_eq!(rss_list.len(), 0);
    }

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
                .content_len(Some(664923008u64))
                .pub_date(Some("2024-01-18T06:57:43.93".to_string()))
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
