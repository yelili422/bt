use bt::rss;
use bt::rss::parsers;
use bt::rss::parsers::MikanParser;
use clap::{Parser, Subcommand};

// The Bangumi Tools CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Rss(RssSubcommand),
}

/// The RSS command
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct RssSubcommand {
    /// Url of rss feed
    #[arg(value_name = "URL")]
    url: String,
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = Cli::parse();

        match args.command {
            Commands::Rss(subcommand) => {
                // TODO: now we only support mikan parser
                let parser = MikanParser::new();
                let rss = rss::Rss::new(subcommand.url, None, &parser);
                match parsers::parse(&rss).await {
                    Ok(feed) => {
                        println!("{:#?}", feed);
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                    }
                }
            }
        }
    });
}
