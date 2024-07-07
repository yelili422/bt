mod mikan;

use super::{Rss, RssSubscription, RssType};
use async_trait::async_trait;
pub use mikan::MikanParser;

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Failed to download {0}: {1}")]
    DownloadFailed(String, String),

    #[error("Invalid RSS: {0}")]
    InvalidRss(String),

    #[error("Unrecognized episode: {0}")]
    UnrecognizedEpisode(String),
}

#[async_trait]
#[allow(async_fn_in_trait)]
pub trait RssParser: Send + Sync {
    fn parse_content(&self, rss: &Rss, content: &str) -> Result<RssSubscription, ParsingError>;

    async fn parse(&self, rss: &Rss) -> Result<RssSubscription, ParsingError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let content = match client.get(&rss.url).send().await {
            Ok(response) => match response.text().await {
                Ok(content) => content,
                Err(err) => {
                    return Err(ParsingError::DownloadFailed(rss.url.clone(), err.to_string()))
                }
            },
            Err(err) => return Err(ParsingError::DownloadFailed(rss.url.clone(), err.to_string())),
        };
        self.parse_content(&rss, &content)
    }
}

pub async fn parse(rss: &Rss) -> Result<RssSubscription, ParsingError> {
    get_parser(&rss.rss_type).parse(rss).await
}

pub fn get_parser(rss_type: &RssType) -> Box<dyn RssParser> {
    match rss_type {
        RssType::Mikan => Box::new(MikanParser::new()),
    }
}
