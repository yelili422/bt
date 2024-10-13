use bt::{
    download_rss_feeds,
    downloader::{self, TaskStatus},
    notification, rename_downloaded_files,
};
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
        #[arg(long, short = 'i', default_value = "300")]
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
            bt::init().await;

            let mut downloader = downloader::DownloadManager::new().await;
            downloader.add_hook(move |status, torrent| {
                if status != TaskStatus::Completed {
                    return;
                }

                let downloading_path_map = downloading_path_map.clone();
                let torrent = torrent.clone();
                let archived_path = archived_path.clone();

                tokio::spawn(async move {
                    let notifier = notification::get_notifier().await;
                    if let Err(err) = rename_downloaded_files(
                        &torrent,
                        &archived_path,
                        downloading_path_map.as_deref(),
                        notifier,
                    )
                    .await
                    {
                        error!("[cmd] Failed to rename downloaded files: {:?}", err);
                    }
                });
            });
            downloader.start();

            tokio::spawn(async move {
                loop {
                    download_rss_feeds(&downloader).await.unwrap_or_else(|e| {
                        error!("[cmd] Failed to fetch RSS feeds: {:?}", e);
                    });
                    debug!("[cmd] Waiting {} seconds for the next update...", interval);
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                }
            });

            tokio::signal::ctrl_c().await?;
            Ok(())
        }
    }
}
