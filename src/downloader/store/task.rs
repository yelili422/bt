use std::path::Path;
use crate::downloader::{DownloadTask, TaskStatus};
use crate::renamer::BangumiInfo;
use log::info;
use sqlx::query;
use std::str::FromStr;

pub async fn add_task(
    pool: &sqlx::SqlitePool,
    task: &DownloadTask,
    bangumi_info: &BangumiInfo,
) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // check if the task is already in the database
    let rec =
        query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, task.torrent_hash,)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some(task) = rec {
        // if the task is already completed or downloading, return the task id
        let task_status = TaskStatus::from_str(&task.status).unwrap();
        if vec![TaskStatus::Completed, TaskStatus::Downloading].contains(&task_status) {
            return Ok(task.id);
        }

        // else remove the task to update all the fields
        query!(r#"DELETE FROM main.download_task WHERE torrent_hash = ?1"#, task.torrent_hash,)
            .execute(&mut *tx)
            .await?;
    }

    let start_time = task.start_time.to_rfc3339();
    let task_status = task.status.to_string();
    let season = bangumi_info.season as i64;
    let episode = bangumi_info.episode as i64;

    let rec = query!(
        r#"
INSERT INTO main.download_task (torrent_hash, torrent_url, start_time, status,
    show_name, episode_name, display_name, season, episode, category, renamed)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
RETURNING id
        "#,
        task.torrent_hash,
        task.torrent_url,
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
    .fetch_one(&mut *tx)
    .await?;

    info!(
        "[store] Add new task [{}-S{:02}E{:02}]({}).",
        bangumi_info.show_name, bangumi_info.season, bangumi_info.episode, task.torrent_hash
    );

    tx.commit().await?;
    Ok(rec.id)
}

pub async fn get_task(
    pool: &sqlx::SqlitePool,
    torrent_hash: &str,
) -> Result<DownloadTask, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let rec = query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, torrent_hash)
        .fetch_one(&mut *tx)
        .await?;

    Ok(DownloadTask {
        id: Some(rec.id),
        torrent_hash: rec.torrent_hash,
        torrent_url: rec.torrent_url,
        start_time: chrono::DateTime::parse_from_rfc3339(&rec.start_time)
            .unwrap()
            .into(),
        status: TaskStatus::from_str(&rec.status).unwrap(),
        renamed: rec.renamed == 1,
    })
}

pub async fn get_tasks_need_renamed(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<DownloadTask>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let status_completed = TaskStatus::Completed.to_string();
    let recs = query!(
        r#"SELECT * FROM main.download_task WHERE status = ?1 AND renamed = 0"#,
        status_completed
    )
    .fetch_all(&mut *tx)
    .await?;

    let tasks = recs
        .iter()
        .map(|rec| DownloadTask {
            id: Some(rec.id),
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

pub async fn get_bangumi_info(
    pool: &sqlx::SqlitePool,
    torrent_hash: &str,
) -> Result<BangumiInfo, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let rec = query!(r#"SELECT * FROM main.download_task WHERE torrent_hash = ?1"#, torrent_hash)
        .fetch_one(&mut *tx)
        .await?;

    Ok(BangumiInfo {
        show_name: rec.show_name,
        episode_name: rec.episode_name,
        display_name: rec.display_name,
        season: rec.season.unwrap_or(1) as u64,
        episode: rec.season.unwrap_or(1) as u64,
        category: rec.category,
    })
}

pub async fn update_task_status(
    pool: &sqlx::SqlitePool,
    torrent_hash: &str,
    status: TaskStatus,
    path: &Path,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let status = status.to_string();
    let download_path = path.display().to_string();
    query!(
        r#"UPDATE main.download_task SET status = ?1 , download_path = ?2 WHERE torrent_hash = ?3"#,
        status,
        download_path,
        torrent_hash
    )
    .execute(&mut *tx)
    .await?;

    info!("[store] Update task [{}] status to {}.", torrent_hash, status);
    tx.commit().await?;
    Ok(())
}
