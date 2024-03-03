mod rss_cmd;

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
    Rss(rss_cmd::RssSubcommand),
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let args = Cli::parse();

        match args.command {
            Commands::Rss(subcommand) => rss_cmd::execute(subcommand).await,
        }
    })
}
