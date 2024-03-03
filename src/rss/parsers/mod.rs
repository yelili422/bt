mod mikan;

use super::{RssSubscription, RssType};
use async_trait::async_trait;
pub use mikan::MikanParser;

#[allow(dead_code)]
#[derive(Debug)]
pub enum ParsingError {
    DownloadFailed(String, String),
    InvalidRss(String),
    UnrecognizedEpisode(String),
}

#[async_trait]
#[allow(async_fn_in_trait)]
pub trait RssParser: Send + Sync {
    fn parse_content(&self, content: &str) -> Result<RssSubscription, ParsingError>;

    async fn parse(&self, url: &str) -> Result<RssSubscription, ParsingError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let content = match client.get(url).send().await {
            Ok(response) => match response.text().await {
                Ok(content) => content,
                Err(err) => {
                    return Err(ParsingError::DownloadFailed(url.to_string(), err.to_string()))
                }
            },
            Err(err) => return Err(ParsingError::DownloadFailed(url.to_string(), err.to_string())),
        };
        self.parse_content(&content)
    }
}

pub async fn parse(rss: &super::Rss) -> Result<RssSubscription, ParsingError> {
    get_parser(&rss.rss_type).parse(rss.url.as_str()).await
}

pub fn get_parser(rss_type: &RssType) -> Box<dyn RssParser> {
    match rss_type {
        RssType::Mikan => Box::new(MikanParser::new()),
    }
}
