use bt::rss::parsers;
use bt::{downloader, renamer, rss};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::path::{Path, PathBuf};

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
        /// Update interval in seconds
        #[arg(long, short, default_value = "300")]
        interval: u64,

        /// Downloading path map
        /// e.g., `/downloads:/mnt/data/downloads`
        #[arg(long, short)]
        downloading_path_map: Option<String>,

        /// Archived directory
        #[arg(long, short)]
        archived_path: String,
    },
}

pub async fn execute(subcommand: DaemonSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        DaemonCommands::Start {
            interval,
            downloading_path_map,
            archived_path,
        } => loop {
            let default_downloader = downloader::get_downloader();

            info!("[cmd] Fetching RSS feeds...");
            let pool = bt::get_pool().await?;
            let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
            for rss in rss_list {
                let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
                match parsers::parse(&rss).await {
                    Ok(feeds) => {
                        for feed in &feeds.items {
                            downloader::download_with_state(
                                default_downloader.as_ref(),
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

            info!("[cmd] Updating task status...");
            // update task status
            let download_list = default_downloader.get_download_list().await?;
            downloader::update_task_status(&download_list).await?;

            info!("[cmd] Renaming completed tasks...");
            // if task is done, rename the file and update the database
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

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        },
    }
}
