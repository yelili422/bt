use log::{debug, error};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::downloader::TorrentMeta;
use crate::rss::parsers::RssParser;
use crate::rss::{Rss, RssSubscription, RssSubscriptionItem};

/// Parse the rss item info from the rss item title.
///
/// The title format is not fixed. A regular format is:
/// [fansub] title1 / title2 / ... - episode [media_info1][media_info2]...
///
/// A rss item info always contains:
/// - fansub
/// - title
/// - season(optional)
/// - episode
/// - media_info
fn parse_rss_item_info(content: &str) -> Option<(String, String, u64, u64, String)> {
    let content = pretreat_rss_item_title(content.to_string());

    // Parsing each item using standard(maybe) format, the result is always correct.
    match split_by_regular_format(&content) {
        Some(captures) => {
            // [喵萌奶茶屋&amp;LoliHouse]
            let fansub = captures
                .name("fansub")
                .map_or("", |m| m.as_str())
                .to_string();
            // 葬送的芙莉莲 / Sousou no Frieren
            let (title, season) =
                parse_bangumi_title_and_season(captures.name("title").map_or("", |m| m.as_str()));
            // 17
            let episode = captures
                .name("episode")
                .unwrap()
                .as_str()
                .parse::<u64>()
                .unwrap();
            // [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
            let media_info = captures
                .name("media")
                .map_or("", |m| m.as_str())
                .to_string();

            let title = remove_redundant_brackets(&title);

            Some((fansub, title, season, episode, media_info))
        }
        None => {
            // If it is fail, fallback to parse every part only and drop some info
            // because we can get them later from database alternatively.
            let (fansub, title, season) = parse_fansub_title_season(&content)?;
            let (episode, media_info) = parse_episode_num_and_media_info(&content)?;

            Some((fansub, title, season, episode, media_info))
        }
    }
}

fn parse_rss_item_torrent(item: &MikanRssItem) -> TorrentMeta {
    TorrentMeta::builder()
        .url(item.enclosure.url.clone())
        .save_path(None)
        .category(None)
        .build()
}

fn pretreat_rss_item_title(title: String) -> String {
    let mut res = title;
    let replace_group = vec![r"【(.*?)】", r"★(.*?)★", r"\*(.*?)\*"];

    for pattern in replace_group.iter() {
        res = Regex::new(pattern)
            .unwrap()
            .replace_all(&res, |caps: &Captures| format!("[{}]", &caps[1]))
            .to_string();
    }

    res = Regex::new(r"\[[^\]]*?月新番\]")
        .unwrap()
        .replace_all(&res, "")
        .to_string();
    res
}

fn parse_rss_item(item: &MikanRssItem) -> Result<RssSubscriptionItem, super::ParsingError> {
    let builder = RssSubscriptionItem::builder()
        .url(item.link.clone())
        .episode_title("".to_string());

    match parse_rss_item_info(&item.title) {
        Some((fansub, title, season, episode, media_info)) => {
            let torrent = parse_rss_item_torrent(item);
            Ok(builder
                .fansub(fansub)
                .title(title)
                .season(season)
                .episode(episode)
                .media_info(media_info)
                .torrent(torrent)
                .category("".to_string())
                .build())
        }
        None => Err(super::ParsingError::UnrecognizedEpisode(format!(
            "Failed to parse rss item: {:?}",
            item
        ))),
    }
}

fn parse_bangumi_title_and_season(content: &str) -> (String, u64) {
    let title_season_re =
        Regex::new(r"([^\[^\]]*)\s第([一|二|三|四|五|六|七|八|九|十]+)季").unwrap();
    let (titles, season) = match title_season_re.captures(content) {
        Some(captures) => {
            let titles = captures.get(1).unwrap().as_str();
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

            (titles, season)
        }
        None => (content, 1),
    };

    let titles: Vec<_> = titles.split(&['/', '|'][..]).collect();
    let title = titles[0].trim();

    (title.to_owned(), season)
}

// e.g. [喵萌奶茶屋&amp;LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
const PATTERN_REGULAR_TITLE: &str = r"^(?x) # enable extend mode
(?<fansub>\[.*?\])
\s*
(?<title>.*?)
\s*\-\s*
(?<episode>\d+)
(?:v\d)?
\s*
(?<episode_name>.*?)?
\s*
(?<media>[\[\(].*[\]\)])*
$";

#[inline]
fn split_by_regular_format(title: &str) -> Option<Captures> {
    let pattern = PATTERN_REGULAR_TITLE;
    let re = Regex::new(&pattern).unwrap();
    return re.captures(title);
}

#[inline]
fn remove_redundant_brackets(title: &str) -> String {
    let re = Regex::new(r"[\[][^\]]*[\]]").unwrap();
    re.replace_all(title, "").trim().to_string()
}

fn parse_fansub_title_season(content: &str) -> Option<(String, String, u64)> {
    let content = content.to_string();

    let slices: Vec<&str> = content
        .split(&['[', ']', '-'][..])
        .filter(|s| !s.is_empty())
        .collect();

    if slices.len() >= 2 {
        let fansub = &format!("[{}]", slices[0]);
        let (title, season) = parse_bangumi_title_and_season(slices[1]);
        Some((fansub.to_string(), title, season))
    } else {
        None
    }
}

fn parse_episode_num_and_media_info(title: &str) -> Option<(u64, String)> {
    let slices: Vec<&str> = title.split(&['[', ']', '-'][..]).collect();

    for (i, s) in slices.iter().enumerate() {
        if let Ok(episode) = s.parse::<u64>() {
            let media_info = &slices[i + 1..]
                .iter()
                .map(|s| {
                    if s.is_empty() {
                        "".to_string()
                    } else {
                        format!("[{}]", s)
                    }
                })
                .collect::<Vec<String>>()
                .concat();
            return Some((episode, media_info.to_owned()));
        }
    }

    None
}

/// strip "Mikan Project - " from title if present
#[inline]
fn strip_mikan_prefix(title: &str) -> &str {
    title.strip_prefix("Mikan Project - ").unwrap_or(title)
}

pub struct MikanParser {}

impl MikanParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl RssParser for MikanParser {
    fn parse_content(
        &self,
        rss: &Rss,
        content: &str,
    ) -> Result<RssSubscription, super::ParsingError> {
        // Parse the content here and return the result
        // If parsing is successful, create and return an RssSubscription
        // If parsing fails, return a ParsingError
        let rss_xml: Result<MikanRss, serde_xml_rs::Error> = serde_xml_rs::from_str(content);
        match rss_xml {
            Ok(rss_xml) => {
                let mut rss_items = Vec::new();

                let raw_title_content =
                    strip_mikan_prefix(rss_xml.channel.title.as_str()).to_string();
                let (channel_title, channel_season) =
                    parse_bangumi_title_and_season(&raw_title_content);

                if channel_title == "我的番组" {
                    debug!("[parser] Parsing aggregation items...");
                }

                for item in rss_xml.channel.item {
                    match parse_rss_item(&item) {
                        Ok(mut rss_item) => {
                            if channel_title != "我的番组" {
                                // PRIORITY: rss title > channel title > item title
                                rss_item.title = channel_title.to_string();
                                rss_item.season = channel_season;

                                if let Some(rss_title) = &rss.title {
                                    rss_item.title = rss_title.to_string();
                                }
                                if let Some(rss_season) = rss.season {
                                    rss_item.season = rss_season;
                                }
                                if let Some(category) = &rss.category {
                                    rss_item.category = category.to_string();
                                }
                            }
                            rss_items.push(rss_item);
                        }
                        Err(err) => {
                            error!("[parser] {}", err);
                        }
                    }
                }

                Ok(RssSubscription {
                    url: rss_xml.channel.link,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct MikanRssItem {
    guid: MikanGuid,
    link: String,
    title: String,
    description: String,
    torrent: MikanTorrent,
    enclosure: MikanEnclosure,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct MikanGuid {
    #[serde(rename = "isPermaLink")]
    is_perma_link: String,
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct MikanTorrent {
    link: String,
    #[serde(rename = "contentLength")]
    content_length: u64,
    #[serde(rename = "pubDate")]
    pub_date: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct MikanEnclosure {
    #[serde(rename = "type")]
    enclosure_type: String,
    length: u64,
    url: String,
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use crate::rss::{Rss, RssBuilder, RssType};
    use crate::{
        downloader::{TorrentMeta, TorrentMetaBuilder},
        rss::{
            parsers::{mikan::MikanParser, RssParser},
            RssSubscription, RssSubscriptionItem,
        },
    };
    use std::fs::read_to_string;

    fn empty_rss() -> Rss {
        Rss::builder()
            .url("".to_string())
            .rss_type(RssType::Mikan)
            .build()
    }

    #[test]
    fn test_parse_rss_item() {
        let titles = vec![
            "[喵萌奶茶屋&amp;LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]",
            "[GJ.Y] 欢迎来到实力至上主义的教室 第三季 / Youkoso Jitsuryoku Shijou Shugi no Kyoushitsu e 3rd Season - 03 (Baha 1920x1080 AVC AAC MP4)",
            "[LoliHouse] 指尖相触，恋恋不舍 / ゆびさきと恋々 / Yubisaki to Renren - 02 [WebRip 1080p HEVC-10bit AAC][简繁内封字幕]",
            "【喵萌奶茶屋】★04月新番★[单间，光照尚好，附带天使。/ワンルーム、日当たり普通、天使つき。/One Room, Hiatari Futsuu, Tenshi-tsuki][01][1080p][简日双语][招募翻译时轴]",
            "[钉铛字幕组]哆啦A梦新番|Doraemon[521][2018.05.18][1080P][附最新的动画组的特效]",
            "【清蓝字幕组】新哆啦A梦 - New Doraemon [437][GB][720P]",
            "[云歌字幕组&萌樱字幕组][4月新番][无名记忆 Unnamed Memory][01][HEVC][x265 10bit][1080p][简体中文][先行版]",
            "[喵萌奶茶屋&LoliHouse] 迷宫饭 / Dungeon Meshi / Delicious in Dungeon - 19v2 [WebRip 1080p HEVC-10bit AAC EAC3][简繁日内封字幕]",
            "[喵萌奶茶屋&LoliHouse] 物语系列 / Monogatari Series: Off & Monster Season - 01 愚物语 [WebRip 1080p HEVC-10bit AAC ASSx2][简繁内封字幕]",
        ];
        let result = vec![
            (
                "[喵萌奶茶屋&amp;LoliHouse]".to_string(),
                "葬送的芙莉莲".to_string(),
                1,
                17,
                "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
            ),
            (
                "[GJ.Y]".to_string(),
                "欢迎来到实力至上主义的教室".to_string(),
                3,
                3,
                "(Baha 1920x1080 AVC AAC MP4)".to_string(),
            ),
            (
                "[LoliHouse]".to_string(),
                "指尖相触，恋恋不舍".to_string(),
                1,
                2,
                "[WebRip 1080p HEVC-10bit AAC][简繁内封字幕]".to_string(),
            ),
            (
                "[喵萌奶茶屋]".to_string(),
                "单间，光照尚好，附带天使。".to_string(),
                1,
                1,
                "[1080p][简日双语][招募翻译时轴]".to_string(),
            ),
            (
                "[钉铛字幕组]".to_string(),
                "哆啦A梦新番".to_string(),
                1,
                521,
                "[2018.05.18][1080P][附最新的动画组的特效]".to_string(),
            ),
            (
                "[清蓝字幕组]".to_string(),
                "新哆啦A梦".to_string(),
                1,
                437,
                "[GB][720P]".to_string(),
            ),
            (
                "[云歌字幕组&萌樱字幕组]".to_string(),
                "无名记忆 Unnamed Memory".to_string(),
                1,
                1,
                "[HEVC][x265 10bit][1080p][简体中文][先行版]".to_string(),
            ),
            (
                "[喵萌奶茶屋&LoliHouse]".to_string(),
                "迷宫饭".to_string(),
                1,
                19,
                "[WebRip 1080p HEVC-10bit AAC EAC3][简繁日内封字幕]".to_string(),
            ),
            (
                "[喵萌奶茶屋&LoliHouse]".to_string(),
                "物语系列".to_string(),
                1,
                1,
                "[WebRip 1080p HEVC-10bit AAC ASSx2][简繁内封字幕]".to_string(),
            ),
        ];

        for (title, expect) in titles.iter().zip(result.iter()) {
            let r = parse_rss_item_info(title);
            assert!(r.is_some(), "title: {}", title);
            assert_eq!(r.unwrap(), *expect, "title: {}", title);
        }
    }

    #[test]
    fn test_parse_fallback_aggregation_rss() {
        // TODO: not supported yet
    }

    #[test]
    fn test_parse_rss_content_normal() {
        let rss_content = read_to_string("./tests/dataset/mikan-1.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&empty_rss(), &rss_content).unwrap();

        let expect = RssSubscription {
            url: "http://mikanani.me/RSS/Bangumi?bangumiId=3141&subgroupid=370".to_string(),
            items: vec![
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/059724511d60173251b378b04709aceff92fffb5".to_string(),
                    title: "葬送的芙莉莲".to_string(),
                    episode_title: "".to_string(),
                    season: 1,
                    episode: 18,
                    fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                    media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                    category: "".to_string(),
                    torrent: TorrentMeta::builder()
                        .url("https://mikanani.me/Download/20240118/059724511d60173251b378b04709aceff92fffb5.torrent".to_string())
                        .build(),
                },
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/872ab5abd72ea223d2a2e36688cc96f83bb71d42".to_string(),
                    title: "葬送的芙莉莲".to_string(),
                    episode_title: "".to_string(),
                    season: 1,
                    episode: 17,
                    fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                    media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                    category: "".to_string(),
                    torrent: TorrentMeta::builder()
                        .url("https://mikanani.me/Download/20240111/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent".to_string())
                        .build(),
                },
            ],
        };
        assert_eq!(res, expect);
    }

    #[test]
    fn test_parse_rss_season_2() {
        let rss_content = read_to_string("./tests/dataset/mikan-2.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&empty_rss(), &rss_content).unwrap();

        let expect = RssSubscription {
            url: "http://mikanani.me/RSS/Bangumi?bangumiId=3223&subgroupid=615".to_string(),
            items: vec![
                RssSubscriptionItem {
                    url: "https://mikanani.me/Home/Episode/65515bee0f9e64d00613e148afac9fbf26e13060".to_string(),
                    title: "弱角友崎同学".to_string(),
                    episode_title: "".to_string(),
                    season: 2,
                    episode: 10,
                    fansub: "[GJ.Y]".to_string(),
                    media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                    category: "".to_string(),
                    torrent: TorrentMeta::builder()
                        .url("https://mikanani.me/Download/20240306/65515bee0f9e64d00613e148afac9fbf26e13060.torrent".to_string())
                        .build(),
                },
            ],
        };
        assert_eq!(res, expect);
    }

    #[test]
    fn test_parse_rss_aggregation() {
        let rss_content = read_to_string("./tests/dataset/mikan-aggregation.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&empty_rss(), &rss_content).unwrap();

        let expect = vec![
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/38b3ab86bc9046f12edca2a2408ac1e7161a8c94".to_string(),
                title: "梦想成为魔法少女".to_string(),
                episode_title: "".to_string(),
                season: 1,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                category: "".to_string(),
                    torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240313/38b3ab86bc9046f12edca2a2408ac1e7161a8c94.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/d2e587e0e10d77fcebdc4552d0725e43e2fa2fe6".to_string(),
                title: "战国妖狐".to_string(),
                episode_title: "".to_string(),
                season: 1,
                episode: 10,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                category: "".to_string(),
                    torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240313/d2e587e0e10d77fcebdc4552d0725e43e2fa2fe6.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/ef56a70e19199829a0280cc022ece291fa186316".to_string(),
                title: "欢迎来到实力至上主义的教室".to_string(),
                episode_title: "".to_string(),
                season: 3,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                category: "".to_string(),
                    torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240313/ef56a70e19199829a0280cc022ece291fa186316.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/49b9c8dd833629d39e09a4e9568bde6b6a71a01b".to_string(),
                title: "弱势角色友崎君".to_string(),
                episode_title: "".to_string(),
                season: 2,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(B-Global 1920x1080 HEVC AAC MKV)".to_string(),
                category: "".to_string(),
                torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240313/49b9c8dd833629d39e09a4e9568bde6b6a71a01b.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/f6d8f1b7131135c2c8b295aca18c64cb6405e2aa".to_string(),
                title: "公主殿下，「拷问」的时间到了".to_string(),
                episode_title: "".to_string(),
                season: 1,
                episode: 10,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                category: "".to_string(),
                torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240312/f6d8f1b7131135c2c8b295aca18c64cb6405e2aa.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/da075c8a8e0b9f71e130b978fb94e4def0745b30".to_string(),
                title: "我内心的糟糕念头".to_string(),
                episode_title: "".to_string(),
                season: 1,
                episode: 22,
                fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                category: "".to_string(),
                torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240310/da075c8a8e0b9f71e130b978fb94e4def0745b30.torrent".to_string())
                    .build(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/6f9bb9e56663194eb68a0811890751d1e66f6fbd".to_string(),
                title: "我独自升级".to_string(),
                episode_title: "".to_string(),
                season: 1,
                episode: 9,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                category: "".to_string(),
                torrent: TorrentMeta::builder()
                    .url("https://mikanani.me/Download/20240310/6f9bb9e56663194eb68a0811890751d1e66f6fbd.torrent".to_string())
                    .build(),
            },
        ];
        res.items.iter().zip(expect.iter()).for_each(|(a, b)| {
            assert_eq!(a, b, "parse rss failed: {}", a.title);
        });
    }
}
