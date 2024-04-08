use bt::downloader::get_downloader;
use bt::{check_downloading_tasks, download_rss_feeds, notification};
use clap::{Parser, Subcommand};
use log::{debug, error};

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
            let downloader = get_downloader();
            let notifier = notification::get_notifier().await;

            let downloader_rss = downloader.clone();
            tokio::spawn(async move {
                loop {
                    download_rss_feeds(downloader_rss.clone())
                        .await
                        .unwrap_or_else(|e| {
                            error!("[cmd] Failed to fetch RSS feeds: {:?}", e);
                        });
                    debug!("[cmd] Waiting {} seconds for the next update...", interval);
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                }
            });

            let downloader_rename = downloader.clone();
            tokio::spawn(async move {
                loop {
                    check_downloading_tasks(
                        downloader_rename.clone(),
                        archived_path.clone(),
                        downloading_path_map.clone(),
                        notifier.clone(),
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
