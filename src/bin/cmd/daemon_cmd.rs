use bt::downloader::Downloader;
use bt::rss::parsers;
use bt::{downloader, renamer, rss};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct DaemonSubcommand {
    #[command(subcommand)]
    command: DaemonCommands,
}

#[derive(Subcommand, Debug)]
enum DaemonCommands {
    /// Start the daemon for fetching RSS feeds and downloading torrents
    Start {
        /// Rss update interval in seconds
        #[arg(long, short, default_value = "300")]
        interval: u64,

        /// Downloading path mapping
        /// Format: `src_path:dst_path`
        /// e.g., `/downloads:/mnt/data/downloads`
        #[arg(long, short = 'm')]
        downloading_path_map: Option<String>,

        /// Archived directory
        /// All completed tasks will be moved to this directory
        #[arg(long, short = 'a')]
        archived_path: String,
    },
}

pub async fn execute(subcommand: DaemonSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        DaemonCommands::Start {
            interval,
            downloading_path_map,
            archived_path,
        } => {
            let downloader = Arc::new(Mutex::from(downloader::get_downloader()));

            let downloader_rss = downloader.clone();
            tokio::spawn(async move {
                loop {
                    fetch_rss_feeds(downloader_rss.clone())
                        .await
                        .unwrap_or_else(|e| {
                            error!("[cmd] Failed to fetch RSS feeds: {:?}", e);
                        });
                    info!("[cmd] Waiting {} seconds for the next update...", interval);
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                }
            });

            let downloader_rename = downloader.clone();
            tokio::spawn(async move {
                loop {
                    process_downloading_tasks(
                        downloader_rename.clone(),
                        archived_path.clone(),
                        downloading_path_map.clone(),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("[cmd] Failed to process downloading tasks: {:?}", e);
                    });
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                }
            });

            tokio::signal::ctrl_c().await?;
            Ok(())
        }
    }
}

pub async fn fetch_rss_feeds(downloader: Arc<Mutex<Box<dyn Downloader>>>) -> anyhow::Result<()> {
    info!("[cmd] Fetching RSS feeds...");
    let pool = bt::get_pool().await?;
    let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
    for rss in rss_list {
        let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
        match parsers::parse(&rss).await {
            Ok(feeds) => {
                for feed in &feeds.items {
                    downloader::download_with_state(
                        downloader.clone(),
                        &feed.torrent,
                        &feed.into(),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("[parser] Failed to download torrent: {:?}", e);
                    });
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
            Err(e) => {
                error!("[parser] Failed to parse RSS: {:?}", e);
            }
        }
    }

    Ok(())
}

pub async fn process_downloading_tasks(
    downloader: Arc<Mutex<Box<dyn Downloader>>>,
    archived_path: String,
    downloading_path_map: Option<String>,
) -> anyhow::Result<()> {
    // update task status
    info!("[cmd] Updating task status...");
    let downloader_lock = downloader.lock().await;
    let download_list = downloader_lock
        .get_download_list()
        .await
        .unwrap_or_default();
    downloader::update_task_status(&download_list).await?;

    // if task is done, rename the file and update the database
    info!("[cmd] Renaming completed tasks...");
    let dst_folder = Path::new(&archived_path);
    for task in download_list {
        if task.status == downloader::TaskStatus::Completed {
            let mut file_path = PathBuf::from(task.save_path).join(task.name);

            if let Some(path_map) = downloading_path_map.as_ref() {
                file_path = renamer::replace_path(file_path, path_map);
            }

            renamer::rename(
                &downloader::get_bangumi_info(&task.hash).await?,
                &file_path,
                dst_folder,
            )?;
            downloader::set_task_renamed(&task.hash).await?;
        }
    }

    Ok(())
}
