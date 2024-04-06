use derive_builder::Builder;
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::downloader::TorrentMeta;
use crate::renamer::{BangumiInfo, BangumiInfoBuilder};
use crate::rss::filter::RssFilterChain;
use crate::tx_begin;

pub mod parsers;
mod store;
mod filter;

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct Rss {
    pub id: Option<i64>,
    pub url: String,
    pub title: Option<String>,
    pub rss_type: RssType,
    pub season: Option<u64>,
    pub enabled: Option<bool>,
    pub filters: Option<RssFilterChain>,
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

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RssSubscriptionItem {
    pub url: String,
    pub title: String,
    pub episode_title: String,
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

pub async fn list_rss() -> anyhow::Result<Vec<Rss>> {
    let mut tx = tx_begin().await?;
    let rss_list = store::query_rss(&mut tx).await?;
    tx.rollback().await?;
    Ok(rss_list)
}

pub async fn add_rss(info: &Rss) -> anyhow::Result<i64> {
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

pub async fn delete_rss(id: i64) -> anyhow::Result<()> {
    let mut tx = tx_begin().await?;
    store::delete_rss(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_rss(id: i64, info: &Rss) -> anyhow::Result<()> {
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

        let mut rss = RssBuilder::default()
            .id(None)
            .title(Some("Sousou no Frieren".to_string()))
            .url("https://mikanani.me/Home/Episode/059724511d60173251b378b04709aceff92fffb5")
            .rss_type(RssType::Mikan)
            .season(1)
            .enabled(true)
            .build()
            .unwrap();

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
}
