use bt::renamer::TvInfo;
use bt::rss::parsers;
use bt::{downloader, rss};
use clap::{Parser, Subcommand};

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
    },
}

pub async fn execute(subcommand: DaemonSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        DaemonCommands::Start { interval } => loop {
            let downloader = downloader::get_downloader();

            let pool = bt::get_pool().await?;
            let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
            for rss in rss_list {
                let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
                let feeds = parsers::parse(&rss).await?;
                for feed in &feeds.items {
                    let rules: TvInfo = feed.into();
                    downloader::download_with_state(
                        downloader.as_ref(),
                        feed.torrent.clone(),
                        rules,
                    )
                    .await?;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        },
    }
}
