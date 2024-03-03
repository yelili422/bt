use derive_builder::Builder;
use log::info;
use sqlx::{query, SqlitePool};

#[derive(Debug, Default, Builder)]
#[builder(setter(into), default)]
pub struct RssEntity {
    id: Option<i64>,
    url: String,
    title: Option<String>,
    rss_type: String,
}

pub async fn add_rss(pool: &SqlitePool, rss: &RssEntity) -> Result<i64, sqlx::Error> {
    // check if the rss url already exists
    let rec = query!(
        r#"
SELECT id
FROM main.rss
WHERE url = ?1
        "#,
        rss.url,
    )
    .fetch_optional(pool)
    .await?;
    if rec.is_some() {
        info!("RSS url {} already exists", &rss.url);
        return Ok(rec.unwrap().id);
    }

    let id = query!(
        r#"
INSERT INTO main.rss (url, title, rss_type)
VALUES (?1, ?2, ?3)
        "#,
        rss.url,
        rss.title,
        rss.rss_type,
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

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
            rss_type: rec.rss_type,
        })
        .collect())
}
