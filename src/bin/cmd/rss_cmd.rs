use bt::rss;
use bt::rss::{parsers, RssType};
use clap::{Parser, Subcommand};
use std::str::FromStr;

/// The RSS command to fetch and manage RSS feeds
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct RssSubcommand {
    #[command(subcommand)]
    command: RssCommands,
}

#[derive(Subcommand, Debug)]
enum RssCommands {
    /// Fetch and display the RSS feed content
    Feed {
        /// Url of rss feed
        #[arg(value_name = "URL")]
        url: String,

        /// Type of rss feed parser.
        ///
        /// ## Supported types
        /// - mikan(default)
        #[arg(long, short, default_value = "mikan")]
        rss_type: String,
    },

    /// Watch the RSS feed for new content
    Watch {
        /// Update interval in seconds
        #[arg(long, short, default_value = "60")]
        interval: u64,
    },

    /// Add a new RSS feed to the bt list
    Add {
        /// Url of rss feed to add
        #[arg(value_name = "URL")]
        url: String,

        /// Type of rss feed parser.
        ///
        /// ## Supported types
        /// - mikan(default)
        #[arg(long, short, default_value = "mikan")]
        rss_type: String,

        /// Title of the rss feed
        ///
        #[arg(long)]
        title: Option<String>,
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

pub async fn execute(subcommand: RssSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        RssCommands::Feed { url, rss_type } => {
            let rss = rss::Rss::new(url, None, RssType::from_str(&rss_type)?);
            serialize_feed(rss).await?;
        }
        RssCommands::Watch { interval } => loop {
            let pool = bt::get_pool().await?;
            let rss_list = rss::store::get_rss_list(&pool).await.unwrap_or_default();
            for rss in rss_list {
                let rss = rss::Rss::new(rss.url, rss.title, rss.rss_type);
                serialize_feed(rss).await?;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        },
        RssCommands::Add {
            url,
            rss_type,
            title,
        } => {
            let rss = rss::store::RssEntityBuilder::default()
                .id(None)
                .url(url)
                .rss_type(RssType::from_str(&rss_type)?)
                .title(title)
                .build()?;
            let pool = bt::get_pool().await?;
            match rss::store::add_rss(&pool, &rss).await {
                Err(e) => {
                    eprintln!("{:?}", e);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
