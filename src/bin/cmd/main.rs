mod daemon_cmd;
mod rss_cmd;

use bt::get_pool;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use log::info;

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

fn main() {
    env_logger::init();

    if let Err(err) = dotenv() {
        info!("Failed to load .env file: {}", err);
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
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
    })
    .expect("Failed to run Tokio runtime");
}
