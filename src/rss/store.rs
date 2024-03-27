use crate::rss::{Rss, RssType};
use sqlx::query;
use std::str::FromStr;

pub async fn check_repeat_by_url(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    url: &str,
) -> Result<Option<i64>, sqlx::Error> {
    // check if the rss url already exists
    let rec = query!(
        r#"
SELECT id
FROM main.rss
WHERE url = ?1
        "#,
        url,
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(rec.map(|rec| rec.id))
}

pub async fn insert_rss(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    rss: &Rss,
) -> Result<i64, sqlx::Error> {
    let rss_type = rss.rss_type.to_string();
    let season = rss.season.map(|s| s as i64);
    let id = query!(
        r#"
INSERT INTO main.rss (url, title, rss_type, enabled, season)
VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        rss.url,
        rss.title,
        rss_type,
        rss.enabled,
        season,
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

pub async fn query_rss(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<Rss>, sqlx::Error> {
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
        .map(|rec| Rss {
            id: Some(rec.id),
            url: rec.url,
            title: rec.title,
            rss_type: RssType::from_str(&rec.rss_type).unwrap(),
            enabled: rec.enabled.map(|e| e == 1),
            season: rec.season.map(|s| s as u64),
        })
        .collect())
}

pub async fn update_rss(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    rss: &Rss,
) -> Result<(), sqlx::Error> {
    let rss_type = rss.rss_type.to_string();
    let season = rss.season.map(|s| s as i64);
    query!(
        r#"
UPDATE main.rss
SET url = ?1, title = ?2, rss_type = ?3, enabled = ?4, season = ?5
WHERE id = ?6
        "#,
        rss.url,
        rss.title,
        rss_type,
        rss.enabled,
        season,
        id,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
