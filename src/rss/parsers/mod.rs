pub mod mikan;

use reqwest::Client;

use super::RssSubscription;

#[derive(Debug)]
pub enum ParsingError {
    DownloadFailed(String, String),
    InvalidRss(String),
    UnrecognizedEpisode(String),
}

pub trait RssParser {
    fn parse_content(&self, content: &str) -> Result<RssSubscription, ParsingError>;

    async fn parse(&self, url: &str) -> Result<RssSubscription, ParsingError> {
        let client = Client::builder()
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
