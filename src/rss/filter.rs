use log::error;
use serde::{Deserialize, Serialize};

use crate::rss::RssSubscriptionItem;

/// RssFilter matches the file names in torrent files, then we can
/// download the matched versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RssFilter {
    /// Match the file name with the given regex.
    FilenameRegex(String),
}

// NOTE: Don't use rss content to match file name, because rss content and rss parsers are not reliable.
async fn match_by_torrent_info_name<F>(rss_item: &RssSubscriptionItem, match_fn: F) -> bool
where
    F: FnOnce(&str) -> bool,
{
    match rss_item.torrent.get_name().await {
        Ok(name) => match_fn(&name),
        Err(err) => {
            error!("[filter] Failed to get torrent name: {:?}", err);
            false
        }
    }
}

fn match_by_regex(regex: &str, filename: &str) -> bool {
    let re = regex::RegexBuilder::new(regex)
        .case_insensitive(true)
        .build();
    match re {
        Ok(re) => re.is_match(filename),
        Err(err) => {
            error!("[filter] Failed to build regex: {:?}", err);
            false
        }
    }
}

impl RssFilter {
    pub async fn is_match(&self, rss_item: &RssSubscriptionItem) -> bool {
        match self {
            RssFilter::FilenameRegex(regex) => {
                match_by_torrent_info_name(rss_item, |name| match_by_regex(regex, name)).await
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RssFilterType {
    Include,
    Exclude,
}

/// RssFilterChain is a chain of filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFilterChain(pub Vec<(RssFilter, RssFilterType)>);

impl RssFilterChain {
    /// Match the given RSS item with all filters.
    ///
    /// There are two types of filters: Include and Exclude.
    /// All downloading items should be included by any `Include` filters(or there is no `Include` filter)
    /// and not excluded by any `Exclude` filters.
    pub async fn is_match(&self, rss_item: &RssSubscriptionItem) -> bool {
        let mut include_matched = false;
        for (filter, filter_type) in &self.0 {
            let matched = filter.is_match(rss_item).await;
            match filter_type {
                RssFilterType::Include => {
                    include_matched |= matched;
                }
                RssFilterType::Exclude => {
                    if matched {
                        return false;
                    }
                }
            }
        }
        include_matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_by_regex() {
        assert!(match_by_regex(
            "CR|Crunchyroll",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (cr 1920x1080 AVC AAC MKV) [5BE12A49].mkv"
        ));
        assert!(match_by_regex(
            "CR|Crunchyroll",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (crunchyroll 1920x1080 AVC AAC MKV) [5BE12A49].mkv"
        ));
        assert!(!match_by_regex(
            "CR|Crunchyroll",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mkv"
        ));

        assert!(match_by_regex(
            r"\.mkv$",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].mkv"
        ));
        assert!(match_by_regex(
            r"\.mkv$",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].MKV"
        ));
        assert!(!match_by_regex(
            r"\.mkv$",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].mp4"
        ));
    }

    fn gen_rss_item_with_filename(filename: &str) -> RssSubscriptionItem {
        use crate::downloader::TorrentMetaBuilder;
        use crate::test::gen_torrent_with_custom_filename;
        use std::sync::Arc;
        use tokio::sync::Mutex;

        RssSubscriptionItem {
            url: "".to_string(),
            title: "".to_string(),
            episode_title: "".to_string(),
            season: 1,
            episode: 1,
            fansub: "".to_string(),
            media_info: "".to_string(),
            torrent: TorrentMetaBuilder::default()
                .url("https://example.com/example.torrent".to_string())
                .data(Arc::new(Mutex::new(Some(gen_torrent_with_custom_filename(filename)))))
                .build()
                .unwrap(),
        }
    }

    #[tokio::test]
    async fn test_filter_chain() {
        let filter_chain = RssFilterChain(vec![
            (RssFilter::FilenameRegex("CR|Crunchyroll".to_string()), RssFilterType::Include),
            (RssFilter::FilenameRegex("Baha".to_string()), RssFilterType::Include),
            (RssFilter::FilenameRegex(r#"\.mp4$"#.to_string()), RssFilterType::Exclude),
        ]);

        let filenames = vec![
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (crunchyroll 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mp4",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (friDay 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
        ];
        let results = vec![true, true, true, false, false];
        for (filename, result) in filenames.iter().zip(results.iter()) {
            let rss_item = gen_rss_item_with_filename(filename);
            assert_eq!(filter_chain.is_match(&rss_item).await, *result);
        }
    }
}
