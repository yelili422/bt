use crate::notification::telegram::Telegram;
use crate::renamer::BangumiInfo;
use async_trait::async_trait;
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::EnumString;
use tokio::sync::{Mutex, OnceCell};

mod telegram;

#[derive(Debug, Clone, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum NotifyType {
    Telegram,
}

pub enum Notification {
    DownloadFinished(BangumiInfo),
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Notification::DownloadFinished(info) => {
                write!(f, "{} download finished.", info.file_name_without_extension())?
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, msg: &str);
}

#[allow(dead_code)]
static GLOBAL_NOTIFIER: OnceCell<Arc<Mutex<Box<dyn Notifier>>>> = OnceCell::const_new();

/// Get the notification type from the environment variable `NOTIFICATION_TYPE`.
/// If the environment variable is not set, return `None`.
fn get_notify_type() -> Option<NotifyType> {
    match std::env::var("NOTIFICATION_TYPE") {
        Ok(notify_type) => NotifyType::from_str(&notify_type).ok(),
        Err(_) => None,
    }
}

/// Get the notifier instance.
pub async fn get_notifier() -> Option<Arc<Mutex<Box<dyn Notifier>>>> {
    let init_fn = |notify_type: NotifyType| async {
        let notifier = match notify_type {
            NotifyType::Telegram => Telegram::from_env(),
        };
        Arc::new(Mutex::new(Box::new(notifier) as Box<dyn Notifier>))
    };

    match get_notify_type() {
        Some(notify_type) => {
            let notifier = GLOBAL_NOTIFIER.get_or_init(|| init_fn(notify_type)).await;
            Some(notifier.clone())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[ignore]
    #[tokio::test]
    async fn test_send_notification() {
        dotenv().ok();

        let notifier = get_notifier().await.unwrap();
        let notifier_lock = notifier.lock().await;
        notifier_lock.send("test").await;
    }
}
