mod daemon_cmd;
mod rss_cmd;

use bt::get_pool;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

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
    _ = dotenv();
    env_logger::init();

    let pool = get_pool().await.expect("Failed to create database pool");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let args = Cli::parse();
    match args.command {
        Commands::Daemon(subcommand) => daemon_cmd::execute(subcommand).await,
        Commands::Rss(subcommand) => rss_cmd::execute(subcommand).await,
    }
    .unwrap();
}
