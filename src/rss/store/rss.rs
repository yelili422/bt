use crate::rss::RssType;
use derive_builder::Builder;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::{query, SqlitePool};
use std::str::FromStr;

#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct RssEntity {
    #[allow(unused)]
    id: Option<i64>,
    pub url: String,
    pub title: Option<String>,
    pub rss_type: RssType,
}

pub async fn add_rss(pool: &SqlitePool, rss: &RssEntity) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // check if the rss url already exists
    let rec = query!(
        r#"
SELECT id
FROM main.rss
WHERE url = ?1
        "#,
        rss.url,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if rec.is_some() {
        info!("[store] RSS url {} already exists", &rss.url);
        return Ok(rec.unwrap().id);
    }

    let rss_type = rss.rss_type.to_string();
    let id = query!(
        r#"
INSERT INTO main.rss (url, title, rss_type)
VALUES (?1, ?2, ?3)
        "#,
        rss.url,
        rss.title,
        rss_type,
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    tx.commit().await?;
    Ok(id)
}

pub async fn get_rss_list(pool: &SqlitePool) -> Result<Vec<RssEntity>, sqlx::Error> {
    let recs = query!(
        r#"
SELECT id, url, title, rss_type
FROM main.rss
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(recs
        .into_iter()
        .map(|rec| RssEntity {
            id: Some(rec.id),
            url: rec.url,
            title: rec.title,
            rss_type: RssType::from_str(&rec.rss_type).unwrap(),
        })
        .collect())
}
