use crate::notification::Notifier;
use async_trait::async_trait;
use log::error;
use teloxide_core::prelude::{Request, Requester};
use teloxide_core::Bot;

pub(crate) struct Telegram {
    token: String,
    chat_id: String,
}

#[allow(unused)]
impl Telegram {
    pub fn new(token: String, chat_id: String) -> Self {
        Self { token, chat_id }
    }

    /// This function is used to create a new instance of the Telegram struct
    /// using the TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID environment variables
    ///
    /// ## Panics
    ///
    /// This function will panic if the TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID
    /// environment variables are not set
    pub fn from_env() -> Self {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN is not set in the environment");

        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .expect("TELEGRAM_CHAT_ID is not set in the environment");

        Self::new(token, chat_id)
    }
}

#[async_trait]
impl Notifier for Telegram {
    async fn send(&self, msg: &str) {
        let bot = Bot::new(self.token.clone());

        match bot.send_message(self.chat_id.clone(), msg).send().await {
            Ok(_) => {}
            Err(err) => {
                error!("[notification] Failed to send message: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification::Notification;
    use crate::renamer::BangumiInfoBuilder;

    #[ignore]
    #[tokio::test]
    async fn test_send() {
        _ = dotenvy::from_path(".env");

        let bangumi_info = BangumiInfoBuilder::default()
            .show_name("test")
            .season(2u64)
            .episode(3u64)
            .build()
            .unwrap();

        let telegram = Telegram::from_env();
        telegram
            .send(&Notification::DownloadFinished(bangumi_info).to_string())
            .await;
    }
}
