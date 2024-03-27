use clap::{Parser, Subcommand};

use bt::{get_pool, init};

mod daemon_cmd;
mod rss_cmd;

// The Bangumi Tools CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Daemon(daemon_cmd::DaemonSubcommand),
    Rss(rss_cmd::RssSubcommand),
}

#[tokio::main]
async fn main() {
    init().await;

    let args = Cli::parse();
    match args.command {
        Commands::Daemon(subcommand) => daemon_cmd::execute(subcommand).await,
        Commands::Rss(subcommand) => rss_cmd::execute(subcommand).await,
    }
    .unwrap();
}
