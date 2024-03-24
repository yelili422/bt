use crate::rss::RssType;
use derive_builder::Builder;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::query;
use std::str::FromStr;

#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct RssEntity {
    #[allow(unused)]
    id: Option<i64>,
    pub url: String,
    pub title: Option<String>,
    pub rss_type: RssType,
    pub enabled: bool,
    pub season: Option<u64>,
}

pub async fn add_rss(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    rss: &RssEntity,
) -> Result<i64, sqlx::Error> {
    // check if the rss url already exists
    let rec = query!(
        r#"
SELECT id
FROM main.rss
WHERE url = ?1
        "#,
        rss.url,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if rec.is_some() {
        info!("[store] RSS url {} already exists", &rss.url);
        return Ok(rec.unwrap().id);
    }

    let rss_type = rss.rss_type.to_string();
    let id = query!(
        r#"
INSERT INTO main.rss (url, title, rss_type, enabled)
VALUES (?1, ?2, ?3, ?4)
        "#,
        rss.url,
        rss.title,
        rss_type,
        rss.enabled,
    )
    .execute(&mut **tx)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn delete_rss(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
) -> Result<(), sqlx::Error> {
    query!(
        r#"
DELETE FROM main.rss
WHERE id = ?1
        "#,
        id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn get_rss_list(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<RssEntity>, sqlx::Error> {
    let recs = query!(
        r#"
SELECT id, url, title, rss_type, enabled, season
FROM main.rss
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(recs
        .into_iter()
        .map(|rec| RssEntity {
            id: Some(rec.id),
            url: rec.url,
            title: rec.title,
            rss_type: RssType::from_str(&rec.rss_type).unwrap(),
            enabled: rec.enabled == 1,
            season: rec.season.map(|s| s as u64),
        })
        .collect())
}
