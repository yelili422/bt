use bt::rss;
use bt::rss::parsers;
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

async fn serialize_feed(rss: rss::Rss) -> anyhow::Result<()> {
    match parsers::parse(&rss).await {
        Ok(feed) => {
            println!("{}", serde_json::to_string(&feed)?);
        }
        Err(e) => {
            eprintln!("{:?}", e);
        }
    }

    Ok(())
}

pub async fn execute(subcommand: DaemonSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        DaemonCommands::Start { interval } => loop {
            let pool = bt::get_pool().await?;
            let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
            for rss in rss_list {
                let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
                serialize_feed(rss).await?;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        },
    }
}
