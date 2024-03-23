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

pub async fn execute(subcommand: RssSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        RssCommands::Feed { url, rss_type } => {
            let rss = rss::RssBuilder::default()
                .url(url)
                .rss_type(RssType::from_str(&rss_type)?)
                .build()
                .unwrap();
            let feeds = parsers::parse(&rss).await?;
            println!("{:?}", feeds)
        }
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
                .enabled(true)
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
