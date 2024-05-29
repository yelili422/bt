use clap::{Parser, Subcommand};

mod daemon_cmd;
mod rss_cmd;
mod torrent_cmd;

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
    Torrent(torrent_cmd::TorrentSubcommand),
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Daemon(subcommand) => daemon_cmd::execute(subcommand).await,
        Commands::Rss(subcommand) => rss_cmd::execute(subcommand).await,
        Commands::Torrent(subcommand) => torrent_cmd::execute(subcommand).await,
    }
    .unwrap();
}
