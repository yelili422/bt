use crate::downloader::{DownloadTask, TaskStatus};
use crate::get_pool;
use crate::renamer::BangumiInfo;
use log::{debug, info};
use sqlx::query;
use std::path::Path;
use std::str::FromStr;

pub async fn add_task(
    rss_id: Option<i64>,
    task: &DownloadTask,
    bangumi_info: &BangumiInfo,
) -> Result<i64, sqlx::Error> {
    let pool = &get_pool().await;
    // check if the task is already in the database
    let rec =
        query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, task.torrent_hash,)
            .fetch_optional(pool)
            .await?;

    if let Some(task) = rec {
        // if the task is already completed or downloading, return the 0
        let task_status = TaskStatus::from_str(&task.status).unwrap();
        if vec![
            TaskStatus::Completed,
            TaskStatus::Downloading,
            TaskStatus::Pause,
        ]
        .contains(&task_status)
        {
            return Ok(0);
        }

        // else remove the task to update all the fields
        query!(r#"DELETE FROM main.download_task WHERE torrent_hash = ?1"#, task.torrent_hash,)
            .execute(pool)
            .await?;
    }

    let start_time = task.start_time.to_rfc3339();
    let task_status = task.status.to_string();
    let season = bangumi_info.season as i64;
    let episode = bangumi_info.episode as i64;

    let rec = query!(
        r#"
INSERT INTO main.download_task (torrent_hash, torrent_url, rss_id, start_time, status,
    show_name, episode_name, display_name, season, episode, category, renamed)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
RETURNING id
        "#,
        task.torrent_hash,
        task.torrent_url,
        rss_id,
        start_time,
        task_status,
        bangumi_info.show_name,
        bangumi_info.episode_name,
        bangumi_info.display_name,
        season,
        episode,
        bangumi_info.category,
        task.renamed,
    )
    .fetch_one(pool)
    .await?;

    info!(
        "[store] Add new task [{}-S{:02}E{:02}]({}).",
        bangumi_info.show_name, bangumi_info.season, bangumi_info.episode, task.torrent_hash
    );

    Ok(rec.id)
}

pub async fn is_task_exist(torrent_url: &str) -> Result<bool, sqlx::Error> {
    let rec = query!(r#"SELECT * FROM main.download_task WHERE torrent_url = ?1"#, torrent_url)
        .fetch_optional(&get_pool().await)
        .await?;

    Ok(rec.is_some())
}

pub async fn get_task(torrent_hash: &str) -> Result<Option<DownloadTask>, sqlx::Error> {
    let rec = query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, torrent_hash)
        .fetch_optional(&get_pool().await)
        .await?;

    match rec {
        None => return Ok(None),
        Some(rec) => Ok(Some(DownloadTask {
            id: Some(rec.id),
            rss_id: rec.rss_id,
            torrent_hash: rec.torrent_hash,
            torrent_url: rec.torrent_url,
            start_time: chrono::DateTime::parse_from_rfc3339(&rec.start_time)
                .unwrap()
                .into(),
            status: TaskStatus::from_str(&rec.status).unwrap(),
            renamed: rec.renamed == 1,
        })),
    }
}

#[allow(unused)]
pub async fn get_tasks_need_renamed() -> Result<Vec<DownloadTask>, sqlx::Error> {
    let status_completed = TaskStatus::Completed.to_string();
    let recs = query!(
        r#"SELECT * FROM main.download_task WHERE status = ?1 AND renamed = 0"#,
        status_completed
    )
    .fetch_all(&get_pool().await)
    .await?;

    let tasks = recs
        .iter()
        .map(|rec| DownloadTask {
            id: Some(rec.id),
            rss_id: rec.rss_id,
            torrent_hash: rec.torrent_hash.clone(),
            torrent_url: rec.torrent_url.clone(),
            start_time: chrono::DateTime::parse_from_rfc3339(&rec.start_time)
                .unwrap()
                .into(),
            status: TaskStatus::from_str(&rec.status).unwrap(),
            renamed: rec.renamed == 1,
        })
        .collect();

    Ok(tasks)
}

pub async fn get_bangumi_info(torrent_hash: &str) -> Result<Option<BangumiInfo>, sqlx::Error> {
    let rec = query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, torrent_hash)
        .fetch_optional(&get_pool().await)
        .await?;

    match rec {
        None => return Ok(None),
        Some(rec) => Ok(Some(BangumiInfo {
            show_name: rec.show_name,
            episode_name: rec.episode_name,
            display_name: rec.display_name,
            season: rec.season.unwrap_or(1) as u64,
            episode: rec.episode.unwrap_or(1) as u64,
            category: rec.category,
        })),
    }
}

pub async fn update_task_status(
    torrent_hash: &str,
    status: TaskStatus,
    path: &Path,
) -> Result<(), sqlx::Error> {
    let status = &status.to_string();
    let download_path = &path.display().to_string();
    query!(
        r#"UPDATE main.download_task SET status = ?1 , download_path = ?2 WHERE torrent_hash = ?3"#,
        status,
        download_path,
        torrent_hash
    )
    .execute(&get_pool().await)
    .await?;

    info!(
        "[store] Updated task [{}]({}) status to {}.",
        torrent_hash, download_path, status
    );
    Ok(())
}

pub async fn update_task_renamed(torrent_hash: &str) -> Result<(), sqlx::Error> {
    query!(
        r#"UPDATE main.download_task SET renamed = 1 WHERE torrent_hash = ?1"#,
        torrent_hash
    )
    .execute(&get_pool().await)
    .await?;

    debug!("[store] Marked task [{}] renamed.", torrent_hash);
    Ok(())
}

pub async fn is_renamed(torrent_hash: &str) -> Result<bool, sqlx::Error> {
    let rec = query!(
        r#"SELECT renamed FROM main.download_task WHERE torrent_hash = ?1"#,
        torrent_hash
    )
    .fetch_one(&get_pool().await)
    .await?;

    Ok(rec.renamed == 1)
}
