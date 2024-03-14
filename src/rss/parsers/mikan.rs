use log::error;
use serde::{Deserialize, Serialize};

use crate::rss::parsers::RssParser;
use crate::{downloader, rss};

pub struct MikanParser {}

impl MikanParser {
    pub fn new() -> Self {
        Self {}
    }
}

const TITLE_PATTERN: &str = r"(.*)\s第([一|二|三|四|五|六|七|八|九|十]+)季";

const EPISODE_TITLE_PATTERN: &str = r"^
(?<fansub>\[.*?\])*
\s*
(?<title>.*?)
\s*\-\s*
(?<episode>\d*)
\s*
(?<media>[\[\(].*[\]\)])*
$";

fn parse_title(title: &str) -> (String, u64) {
    let re = regex::Regex::new(TITLE_PATTERN).unwrap();
    match re.captures(title) {
        Some(captures) => {
            let title = captures.get(1).unwrap().as_str();
            let season = captures.get(2).unwrap().as_str();
            let season = match season {
                "一" => 1,
                "二" => 2,
                "三" => 3,
                "四" => 4,
                "五" => 5,
                "六" => 6,
                "七" => 7,
                "八" => 8,
                "九" => 9,
                "十" => 10,
                _ => unimplemented!("implemented season number"),
            };
            (title.to_string(), season)
        }
        None => (title.to_string(), 1),
    }
}

fn parser_episode_title(title: &str) -> Option<regex::Captures> {
    let pattern = EPISODE_TITLE_PATTERN.replace("\n", "");
    let re = regex::Regex::new(&pattern).unwrap();
    return re.captures(title);
}

impl RssParser for MikanParser {
    fn parse_content(&self, content: &str) -> Result<rss::RssSubscription, super::ParsingError> {
        // Parse the content here and return the result
        // If parsing is successful, create and return an RssSubscription
        // If parsing fails, return a ParsingError
        let rss: Result<MikanRss, serde_xml_rs::Error> = serde_xml_rs::from_str(content);
        match rss {
            Ok(rss) => {
                let mut rss_items = Vec::new();

                let raw_title = rss.channel.title.as_str();
                // strip "Mikan Project - " from title if present
                let row_title_content = raw_title
                    .strip_prefix("Mikan Project - ")
                    .unwrap_or(raw_title)
                    .to_string();
                let (mut title, season) = parse_title(&row_title_content);

                for item in rss.channel.item {
                    let mut episode = 1u64;
                    let mut fansub = "";
                    let mut media_info = "";

                    // e.g. [喵萌奶茶屋&amp;LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
                    let rss_title = item.title;

                    match parser_episode_title(&rss_title) {
                        Some(captures) => {
                            // [喵萌奶茶屋&amp;LoliHouse]
                            if let Some(fansub_part) = captures.name("fansub") {
                                fansub = fansub_part.as_str();
                            }
                            // 葬送的芙莉莲 / Sousou no Frieren
                            if let Some(title_part) = captures.name("title") {
                                let title_part = title_part.as_str();
                                let titles: Vec<_> = title_part.split('/').collect();

                                // prioritize the title in the rss header
                                if title == "" && titles.len() > 1 {
                                    title = titles[0].trim().to_string();
                                }
                            }
                            // 17
                            if let Some(episode_part) = captures.name("episode") {
                                episode = episode_part.as_str().parse::<u64>().unwrap();
                            }
                            // [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
                            if let Some(media_part) = captures.name("media") {
                                media_info = media_part.as_str();
                            }
                        }
                        None => {
                            error!("Unrecognized episode: {}", rss_title);

                            // Ignore the unrecognized episode and continue
                            // return Err(super::ParsingError::UnrecognizedEpisode(
                            //     rss_title.to_string(),
                            // ));
                        }
                    }

                    let torrent = downloader::TorrentMetaBuilder::default()
                        .url(item.enclosure.url)
                        .content_len(item.enclosure.length)
                        .pub_date(item.torrent.pub_date)
                        .build()
                        .unwrap();

                    let rss_item = rss::RssSubscriptionItem {
                        url: item.link,
                        title: title.to_string(),
                        episode_title: "".to_string(),
                        description: item.description,
                        season,
                        episode,
                        fansub: fansub.to_string(),
                        media_info: media_info.to_string(),
                        torrent,
                    };
                    rss_items.push(rss_item);
                }

                Ok(rss::RssSubscription {
                    url: rss.channel.link,
                    description: rss.channel.description,
                    items: rss_items,
                })
            }
            Err(err) => Err(super::ParsingError::InvalidRss(err.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanRss {
    channel: MikanRssChannel,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanRssChannel {
    title: String,
    link: String,
    description: String,
    item: Vec<MikanRssItem>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanRssItem {
    guid: MikanGuid,
    link: String,
    title: String,
    description: String,
    torrent: MikanTorrent,
    enclosure: MikanEnclosure,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanGuid {
    #[serde(rename = "isPermaLink")]
    is_perma_link: String,
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanTorrent {
    link: String,
    #[serde(rename = "contentLength")]
    content_length: u64,
    #[serde(rename = "pubDate")]
    pub_date: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MikanEnclosure {
    #[serde(rename = "type")]
    enclosure_type: String,
    length: u64,
    url: String,
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use crate::{
        downloader::{TorrentMeta, TorrentMetaBuilder},
        rss::{
            parsers::{mikan::MikanParser, RssParser},
            RssSubscription, RssSubscriptionItem,
        },
    };
    use std::fs::read_to_string;

    #[test]
    fn parse_episode_title() {
        for title in vec![
            "[喵萌奶茶屋&amp;LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]",
            "[GJ.Y] 欢迎来到实力至上主义的教室 第三季 / Youkoso Jitsuryoku Shijou Shugi no Kyoushitsu e 3rd Season - 03 (Baha 1920x1080 AVC AAC MP4)",
            "[LoliHouse] 指尖相触，恋恋不舍 / ゆびさきと恋々 / Yubisaki to Renren - 02 [WebRip 1080p HEVC-10bit AAC][简繁内封字幕]",
        ] {
            assert!(super::parser_episode_title(title).is_some());
        }
    }

    #[test]
    fn parse_rss_content_normal() {
        let rss_content = read_to_string("./tests/dataset/mikan-1.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&rss_content).unwrap();

        let expect = RssSubscription {
            url: "http://mikanani.me/RSS/Bangumi?bangumiId=3141&subgroupid=370".to_string(),
            description: "Mikan Project - 葬送的芙莉莲".to_string(),
            items: vec![
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/059724511d60173251b378b04709aceff92fffb5".to_string(),
                    title: "葬送的芙莉莲".to_string(),
                    episode_title: "".to_string(),
                    description: "[喵萌奶茶屋&LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 18 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕][634.12 MB]".to_string(),
                    season: 1,
                    episode: 18,
                    fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                    media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                    torrent: TorrentMetaBuilder::default()
                        .url("https://mikanani.me/Download/20240118/059724511d60173251b378b04709aceff92fffb5.torrent")
                        .content_len(664923008u64)
                        .pub_date("2024-01-18T06:57:43.93")
                        .build()
                        .unwrap(),
                },
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/872ab5abd72ea223d2a2e36688cc96f83bb71d42".to_string(),
                    title: "葬送的芙莉莲".to_string(),
                    episode_title: "".to_string(),
                    description: "[喵萌奶茶屋&LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕][639.78 MB]".to_string(),
                    season: 1,
                    episode: 17,
                    fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                    media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                    torrent: TorrentMetaBuilder::default()
                        .url("https://mikanani.me/Download/20240111/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent")
                        .content_len(670857984u64)
                        .pub_date("2024-01-11T06:57:59.057")
                        .build()
                        .unwrap(),
                },
            ]
        };
        assert_eq!(res, expect)
    }

    #[test]
    fn parse_rss_season_2() {
        let rss_content = read_to_string("./tests/dataset/mikan-2.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&rss_content).unwrap();

        let expect = RssSubscription {
            url: "http://mikanani.me/RSS/Bangumi?bangumiId=3223&subgroupid=615".to_string(),
            description: "Mikan Project - 弱角友崎同学 第二季".to_string(),
            items: vec![
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/65515bee0f9e64d00613e148afac9fbf26e13060".to_string(),
                    title: "弱角友崎同学".to_string(),
                    episode_title: "".to_string(),
                    description: "[GJ.Y] 弱角友崎同学 2nd STAGE / Jaku-Chara Tomozaki-kun 2nd Stage - 10 (Baha 1920x1080 AVC AAC MP4)[428.25 MB]".to_string(),
                    season: 2,
                    episode: 10,
                    fansub: "[GJ.Y]".to_string(),
                    media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                    torrent: TorrentMetaBuilder::default()
                        .url("https://mikanani.me/Download/20240306/65515bee0f9e64d00613e148afac9fbf26e13060.torrent")
                        .content_len(449052672u64)
                        .pub_date("2024-03-06T21:41:22.281")
                        .build()
                        .unwrap(),
                },
            ]
        };
        assert_eq!(res, expect)
    }
}
