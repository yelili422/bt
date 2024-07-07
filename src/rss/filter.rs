use log::error;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::rss::RssSubscriptionItem;

/// RssFilter matches the file names in torrent files, then we can
/// download the matched versions.
#[derive(Debug, Clone)]
pub enum RssFilter {
    /// Match the file name with the given regex.
    FilenameRegex(String),
}

impl Serialize for RssFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RssFilter::FilenameRegex(regex) => {
                serializer.serialize_str(&format!("FilenameRegex-{}", regex))
            }
        }
    }
}

struct RssFilterVisitor;

impl<'de> Visitor<'de> for RssFilterVisitor {
    type Value = RssFilter;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string like 'FilenameRegex-<regex>'")
    }
}

impl<'de> Deserialize<'de> for RssFilter {
    fn deserialize<D>(deserializer: D) -> Result<RssFilter, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let parts: Vec<&str> = s.splitn(2, '-').collect();
        if parts.len() < 2 {
            return Err(serde::de::Error::custom("Invalid filter format"));
        }

        let rest = parts[1..].join("-");

        match parts[0] {
            "FilenameRegex" => Ok(RssFilter::FilenameRegex(rest)),
            _ => Err(serde::de::Error::custom("Invalid filter type")),
        }
    }
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

/// RssFilterChain is a chain of filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFilterChain(pub Vec<RssFilter>);

impl RssFilterChain {
    /// Match the given RSS item with all filters.
    pub async fn is_match(&self, rss_item: &RssSubscriptionItem) -> bool {
        for filter in &self.0 {
            if filter.is_match(rss_item).await {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::downloader::TorrentMeta;

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

    async fn gen_rss_item_with_filename(filename: &str) -> RssSubscriptionItem {
        use crate::downloader::update_torrent_cache;
        use crate::test::gen_torrent_with_custom_filename;

        let url = &format!("https://example.com/example.torrent?filename={}", filename);
        let torrent = gen_torrent_with_custom_filename(filename);
        update_torrent_cache(url, &torrent).await;

        RssSubscriptionItem {
            url: "".to_string(),
            title: "".to_string(),
            episode_title: "".to_string(),
            season: 1,
            episode: 1,
            fansub: "".to_string(),
            media_info: "".to_string(),
            torrent: TorrentMeta::builder()
                .url(url.to_string())
                .category(None)
                .save_path(None)
                .pub_date(None)
                .content_len(None)
                .build(),
            category: "".to_string(),
        }
    }

    #[tokio::test]
    async fn test_filter_chain() {
        let filter_chain = RssFilterChain(vec![
            RssFilter::FilenameRegex("CR|Crunchyroll".to_string()),
            RssFilter::FilenameRegex("Baha".to_string()),
            RssFilter::FilenameRegex(r#"\.mp4$"#.to_string()),
        ]);

        let filenames = vec![
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (crunchyroll 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mp4",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (friDay 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
        ];
        let results = vec![false, false, false, false, true];
        for (filename, result) in filenames.iter().zip(results.iter()) {
            let rss_item = gen_rss_item_with_filename(filename).await;
            assert_eq!(filter_chain.is_match(&rss_item).await, *result, "{}", filename);
        }
    }

    #[tokio::test]
    async fn test_filter_chain_exclude_only() {
        let filter_chain = RssFilterChain(vec![RssFilter::FilenameRegex(r#"\.mp4$"#.to_string())]);

        let filenames = vec![
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (CR 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (crunchyroll 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mkv",
            "[Up to 21°C] Yuru Camp△ Season 3 - 01 (Baha 1920x1080 AVC AAC MKV) [5BE12A49].mp4",
        ];
        let results = vec![true, true, true, false];
        for (filename, result) in filenames.iter().zip(results.iter()) {
            let rss_item = gen_rss_item_with_filename(filename).await;
            assert_eq!(filter_chain.is_match(&rss_item).await, *result, "{}", filename);
        }
    }
}
