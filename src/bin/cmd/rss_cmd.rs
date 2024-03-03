use bt::rss;
use bt::rss::parsers;
use bt::rss::parsers::MikanParser;
use clap::{Parser, Subcommand};

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
        rss_type: Option<String>,
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

pub async fn execute(subcommand: RssSubcommand) {
    match subcommand.command {
        RssCommands::Feed { url, .. } => {
            // TODO: now we only support mikan parser
            let parser = MikanParser::new();
            let rss = rss::Rss::new(url, None, &parser);
            match parsers::parse(&rss).await {
                Ok(feed) => {
                    println!("{:#?}", feed);
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        }
        RssCommands::Add { url, rss_type, title } => {
            let rss = rss::store::RssEntityBuilder::default()
                .url(url)
                .rss_type(rss_type)
                .title(title)
                .build()
                .unwrap();
            let pool = bt::get_pool().await.expect("Failed to get db pool");
            match rss::store::add_rss(&pool, &rss).await {
                Err(e) => {
                    eprintln!("{:?}", e);
                },
                _ => {}
            }
        }
    }
}
