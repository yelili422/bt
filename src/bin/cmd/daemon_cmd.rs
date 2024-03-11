use bt::rss::parsers;
use bt::{downloader, renamer, rss};
use clap::{Parser, Subcommand};
use std::path::Path;

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

        /// Archived directory
        #[arg(long, short)]
        destination: String,
    },
}

pub async fn execute(subcommand: DaemonSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        DaemonCommands::Start {
            interval,
            destination,
        } => loop {
            let default_downloader = downloader::get_downloader();

            let pool = bt::get_pool().await?;
            let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
            for rss in rss_list {
                let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
                let feeds = parsers::parse(&rss).await?;
                for feed in &feeds.items {
                    downloader::download_with_state(
                        default_downloader.as_ref(),
                        &feed.torrent,
                        &feed.into(),
                    )
                    .await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }

            // update task status
            let download_list = default_downloader.get_download_list().await?;
            downloader::update_task_status(&download_list).await?;

            // if task is done, rename the file and update the database
            let dst_folder = Path::new(&destination);
            for task in download_list {
                if task.status == downloader::TaskStatus::Completed {
                    renamer::rename(
                        &downloader::get_bangumi_info(&task.hash).await?,
                        &task.get_file_path(),
                        dst_folder,
                    )?;
                    downloader::set_task_renamed(&task.hash).await?;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        },
    }
}
