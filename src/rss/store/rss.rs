use sqlx::{SqlitePool, query};

pub struct RssEntity {
    id: i64,
    url: String,
    title: Option<String>,
}

pub async fn add_rss(
    pool: &SqlitePool,
    rss: &RssEntity,
) -> Result<i64, sqlx::Error> {
    let id = query!(
        r#"
INSERT INTO main.rss (url, title)
VALUES (?1, ?2)
        "#,
        rss.url,
        rss.title,
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn get_rss_list(
    pool: &SqlitePool,
) -> Result<Vec<RssEntity>, sqlx::Error> {
    let recs = query!(
        r#"
SELECT id, url, title
FROM main.rss
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(recs
        .into_iter()
        .map(|rec| RssEntity {
            id: rec.id,
            url: rec.url,
            title: rec.title,
        })
        .collect())
}
