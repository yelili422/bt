use clap::{Parser, Subcommand};

// The Bangumi Tools CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
enum Commands {
    Rss(RssSubcommand)
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
                match bt::rss::parsers::parse(&subcommand.url).await {
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
