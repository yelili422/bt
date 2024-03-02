mod mikan;

use super::RssSubscription;
pub use mikan::MikanParser;

#[allow(dead_code)]
#[derive(Debug)]
pub enum ParsingError {
    DownloadFailed(String, String),
    InvalidRss(String),
    UnrecognizedEpisode(String),
}

#[allow(async_fn_in_trait)]
pub trait RssParser {
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

pub async fn parse<'a, P>(rss: &super::Rss<'a, P>) -> Result<RssSubscription, ParsingError>
where
    P: RssParser,
{
    rss.parser.parse(&rss.url).await
}
