use chrono::{DateTime, Local};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DownloadTask {
    pub id: Option<i64>,
    pub torrent_hash: String,
    pub torrent_url: Option<String>,
    pub start_time: DateTime<Local>,
    pub status: TaskStatus,
    pub renamed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pause,
    Error,
    Downloading,
    Completed,
}
