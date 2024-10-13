use crate::{
    get_pool,
    rss::{Rss, RssType},
    tx_begin,
};
use log::info;
use sqlx::query;
use std::str::FromStr;

use super::filter::RssFilterChain;

pub async fn add_rss(info: &Rss) -> Result<i64, sqlx::Error> {
    let tx = tx_begin().await?;

    let id = match check_repeat_by_url(&info.url).await? {
        Some(id) => {
            info!("[store] RSS url {} already exists", &info.url);
            id
        }
        None => insert_rss(&info).await?,
    };

    tx.commit().await?;
    Ok(id)
}

pub async fn check_repeat_by_url(url: &str) -> Result<Option<i64>, sqlx::Error> {
    // check if the rss url already exists
    let rec = query!(
        r#"
SELECT id
FROM main.rss
WHERE url = ?1
        "#,
        url,
    )
    .fetch_optional(&get_pool().await)
    .await?;

    Ok(rec.map(|rec| rec.id))
}

fn serialize_filters(rss_filters: &Option<RssFilterChain>) -> Option<String> {
    match &rss_filters {
        Some(f) => Some(serde_json::to_string(f).unwrap()),
        None => None,
    }
}

fn deserialize_filters(filters_str: &Option<String>) -> Option<RssFilterChain> {
    match filters_str {
        Some(filters_str) => Some(serde_json::from_str(filters_str).unwrap()),
        None => None,
    }
}

pub async fn insert_rss(rss: &Rss) -> Result<i64, sqlx::Error> {
    let rss_type = rss.rss_type.to_string();
    let season = rss.season.map(|s| s as i64);
    let filters = serialize_filters(&rss.filters);
    let id = query!(
        r#"
INSERT INTO main.rss (url, title, rss_type, enabled, season, filters, description, category)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        rss.url,
        rss.title,
        rss_type,
        rss.enabled,
        season,
        filters,
        rss.description,
        rss.category,
    )
    .execute(&get_pool().await)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn delete_rss(id: i64) -> Result<(), sqlx::Error> {
    query!(
        r#"
DELETE FROM main.rss
WHERE id = ?1
        "#,
        id,
    )
    .execute(&get_pool().await)
    .await?;

    Ok(())
}

pub async fn query_rss() -> Result<Vec<Rss>, sqlx::Error> {
    let recs = query!(
        r#"
SELECT id, url, title, rss_type, enabled, season, filters, description, category
FROM main.rss
ORDER BY enabled DESC, title ASC, season ASC
        "#,
    )
    .fetch_all(&get_pool().await)
    .await?;

    Ok(recs
        .into_iter()
        .map(|rec| Rss {
            id: Some(rec.id),
            url: rec.url,
            title: rec.title,
            rss_type: RssType::from_str(&rec.rss_type).unwrap(),
            enabled: rec.enabled.map(|e| e == 1),
            season: rec.season.map(|s| s as u64),
            filters: deserialize_filters(&rec.filters),
            description: rec.description,
            category: rec.category,
        })
        .collect())
}

pub async fn update_rss(id: i64, rss: &Rss) -> Result<(), sqlx::Error> {
    let rss_type = rss.rss_type.to_string();
    let season = rss.season.map(|s| s as i64);
    let filters = serialize_filters(&rss.filters);
    query!(
        r#"
UPDATE main.rss
SET url = ?1, title = ?2, rss_type = ?3, enabled = ?4, season = ?5, filters = ?6, description = ?7, category = ?8
WHERE id = ?9
        "#,
        rss.url,
        rss.title,
        rss_type,
        rss.enabled,
        season,
        filters,
        rss.description,
        rss.category,
        id,
    )
    .execute(&get_pool().await)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;

    #[tokio::test]
    async fn test_rss() {
        init().await;

        let rss_list = query_rss().await.unwrap();
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
        let rss_list = query_rss().await.unwrap();
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

        let rss_list = query_rss().await.unwrap();
        assert_eq!(rss_list.len(), 1);
        assert_eq!(rss_list[0].title, rss.title);

        delete_rss(id).await.unwrap();
        let rss_list = query_rss().await.unwrap();
        assert_eq!(rss_list.len(), 0);
    }
}
