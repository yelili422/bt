use crate::renamer::BangumiInfo;
use chrono::{DateTime, Local};
use derive_builder::Builder;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::query;
use strum_macros::{Display, EnumString};

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DownloadTask {
    pub id: Option<i64>,
    pub torrent_hash: String,
    pub torrent_url: Option<String>,
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum TaskStatus {
    Downloading,
    Completed,
}

pub async fn add_task(
    pool: &sqlx::SqlitePool,
    task: &DownloadTask,
    bangumi_info: &BangumiInfo,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // check if the task is already in the database
    let downloading = TaskStatus::Downloading.to_string();
    let rec = query!(
        r#"
SELECT * FROM main.download_task WHERE torrent_hash = ?1 AND status = ?2
        "#,
        task.torrent_hash,
        downloading
    )
    .fetch_optional(&mut *tx)
    .await?;

    if rec.is_some() {
        tx.commit().await?;
        return Ok(());
    }

    let start_time = task.start_time.to_rfc3339();
    let end_time = task.end_time.map(|t| t.to_rfc3339());
    let task_status = task.status.to_string();
    let season = bangumi_info.season as i64;
    let episode = bangumi_info.episode as i64;
    query!(
        r#"
INSERT INTO main.download_task (torrent_hash, torrent_url, start_time, end_time, status,
    show_name, episode_name, display_name, season, episode, category)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        task.torrent_hash,
        task.torrent_url,
        start_time,
        end_time,
        task_status,
        bangumi_info.show_name,
        bangumi_info.episode_name,
        bangumi_info.display_name,
        season,
        episode,
        bangumi_info.category
    )
    .execute(&mut *tx)
    .await?;

    info!(
        "[store] Add new task [{}-S{:02}E{:02}]({}).",
        bangumi_info.show_name, bangumi_info.season, bangumi_info.episode, task.torrent_hash
    );

    tx.commit().await?;
    Ok(())
}
