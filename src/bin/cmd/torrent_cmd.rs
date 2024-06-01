use bt::downloader::Torrent;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct TorrentSubcommand {
    #[command(subcommand)]
    command: TorrentCommands,
}

#[derive(Subcommand, Debug)]
enum TorrentCommands {
    /// Compute the torrent id.
    ID {
        /// The torrent file path.
        file: Option<String>,
    },
}

pub async fn execute(subcommand: TorrentSubcommand) -> anyhow::Result<()> {
    match subcommand.command {
        TorrentCommands::ID { file } => {
            let dot_torrent = std::fs::read(file.unwrap()).unwrap();
            let torrent = Torrent::from_bytes(&dot_torrent).unwrap();
            let info_hash = torrent.torrent_id();
            println!("{}", hex::encode(info_hash));
        }
    }
    Ok(())
}
