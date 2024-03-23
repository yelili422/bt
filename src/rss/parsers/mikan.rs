use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::rss::parsers::RssParser;
use crate::rss::Rss;
use crate::{downloader, rss};

pub struct MikanParser {}

impl MikanParser {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_rss_item(
        &self,
        item: &MikanRssItem,
    ) -> Result<rss::RssSubscriptionItem, super::ParsingError> {
        let mut title = String::new();
        let mut episode = 1u64;
        let mut season = 1u64;
        let mut fansub = String::new();
        let mut media_info = String::new();

        // e.g. [喵萌奶茶屋&amp;LoliHouse] 葬送的芙莉莲 / Sousou no Frieren - 17 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
        let rss_title = item.title.clone();

        match parser_episode_title(&rss_title) {
            Some(captures) => {
                // [喵萌奶茶屋&amp;LoliHouse]
                if let Some(fansub_part) = captures.name("fansub") {
                    fansub = fansub_part.as_str().to_string();
                }
                // 葬送的芙莉莲 / Sousou no Frieren
                if let Some(title_part) = captures.name("title") {
                    let (_title, _season) = parse_bangumi_title(title_part.as_str());
                    (title, season) = (_title.to_string(), _season);

                    let bangumi_names: Vec<_> = title.split('/').collect();
                    let re = Regex::new(r"\[(.*?)\]").unwrap();
                    title = re.replace_all(bangumi_names[0], "").to_string();
                    title = title.trim().to_string();
                }
                // 17
                if let Some(episode_part) = captures.name("episode") {
                    episode = episode_part.as_str().parse::<u64>().unwrap();
                }
                // [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]
                if let Some(media_part) = captures.name("media") {
                    media_info = media_part.as_str().to_string();
                }
            }
            None => {
                return Err(super::ParsingError::UnrecognizedEpisode(rss_title.to_string()));
            }
        }

        let torrent = downloader::TorrentMetaBuilder::default()
            .url(&item.enclosure.url)
            .content_len(item.enclosure.length)
            .pub_date(&item.torrent.pub_date)
            .build()
            .unwrap();

        Ok(rss::RssSubscriptionItem {
            url: item.link.clone(),
            title,
            episode_title: "".to_string(),
            description: item.description.clone(),
            season,
            episode,
            fansub,
            media_info,
            torrent,
        })
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

fn parse_bangumi_title(title: &str) -> (&str, u64) {
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

            (title, season)
        }
        None => (title, 1),
    }
}

fn parser_episode_title(title: &str) -> Option<regex::Captures> {
    let pattern = EPISODE_TITLE_PATTERN.replace("\n", "");
    let re = regex::Regex::new(&pattern).unwrap();
    return re.captures(title);
}

/// strip "Mikan Project - " from title if present
fn strip_mikan_prefix(title: &str) -> &str {
    title.strip_prefix("Mikan Project - ").unwrap_or(title)
}

impl RssParser for MikanParser {
    fn parse_content(
        &self,
        rss: &Rss,
        content: &str,
    ) -> Result<rss::RssSubscription, super::ParsingError> {
        // Parse the content here and return the result
        // If parsing is successful, create and return an RssSubscription
        // If parsing fails, return a ParsingError
        let rss_xml: Result<MikanRss, serde_xml_rs::Error> = serde_xml_rs::from_str(content);
        match rss_xml {
            Ok(rss_xml) => {
                let mut rss_items = Vec::new();

                let raw_title_content =
                    strip_mikan_prefix(rss_xml.channel.title.as_str()).to_string();
                let (channel_title, channel_season) = parse_bangumi_title(&raw_title_content);

                for item in rss_xml.channel.item {
                    match self.parse_rss_item(&item) {
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
                            } else {
                                // use default item title
                            }
                            rss_items.push(rss_item);
                        }
                        Err(err) => {
                            error!("[parser] {}", err);
                        }
                    }
                }

                Ok(rss::RssSubscription {
                    url: rss_xml.channel.link,
                    description: rss_xml.channel.description,
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
    use crate::rss::{Rss, RssBuilder, RssType};
    use crate::{
        downloader::{TorrentMeta, TorrentMetaBuilder},
        rss::{
            parsers::{mikan::MikanParser, RssParser},
            RssSubscription, RssSubscriptionItem,
        },
    };
    use std::fs::read_to_string;

    fn mock_rss() -> Rss {
        RssBuilder::default()
            .url("")
            .rss_type(RssType::Mikan)
            .title(None)
            .season(None)
            .build()
            .unwrap()
    }

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
        let res = parser.parse_content(&mock_rss(), &rss_content).unwrap();

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
            ],
        };
        assert_eq!(res, expect)
    }

    #[test]
    fn parse_rss_season_2() {
        let rss_content = read_to_string("./tests/dataset/mikan-2.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&mock_rss(), &rss_content).unwrap();

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
            ],
        };
        assert_eq!(res, expect)
    }

    #[test]
    fn parse_rss_aggregation() {
        let rss_content = read_to_string("./tests/dataset/mikan-aggregation.rss").unwrap();
        assert_ne!(&rss_content, "");

        let parser = MikanParser::new();
        let res = parser.parse_content(&mock_rss(), &rss_content).unwrap();

        let expect = vec![
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/38b3ab86bc9046f12edca2a2408ac1e7161a8c94".to_string(),
                title: "梦想成为魔法少女".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 梦想成为魔法少女 [年龄限制版] / Mahou Shoujo ni Akogarete - 11 (Baha 1920x1080 AVC AAC MP4)[528.96 MB]".to_string(),
                season: 1,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240313/38b3ab86bc9046f12edca2a2408ac1e7161a8c94.torrent")
                    .content_len(554654784u64)
                    .pub_date("2024-03-13T23:31:32.102")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/d2e587e0e10d77fcebdc4552d0725e43e2fa2fe6".to_string(),
                title: "战国妖狐".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 战国妖狐 / Sengoku Youko - 10 (Baha 1920x1080 AVC AAC MP4)[624.03 MB]".to_string(),
                season: 1,
                episode: 10,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(Baha 1920x1080 AVC AAC MP4)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240313/d2e587e0e10d77fcebdc4552d0725e43e2fa2fe6.torrent")
                    .content_len(654342912u64)
                    .pub_date("2024-03-13T23:02:04.724")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/ef56a70e19199829a0280cc022ece291fa186316".to_string(),
                title: "欢迎来到实力至上主义的教室".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 欢迎来到实力至上主义的教室 第三季 / Youkoso Jitsuryoku Shijou Shugi no Kyoushitsu e 3rd Season - 11 (CR 1920x1080 AVC AAC MKV)[1.37 GB]".to_string(),
                season: 3,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240313/ef56a70e19199829a0280cc022ece291fa186316.torrent")
                    .content_len(1471026304u64)
                    .pub_date("2024-03-13T22:01:57.497")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/49b9c8dd833629d39e09a4e9568bde6b6a71a01b".to_string(),
                title: "弱势角色友崎君".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 弱势角色友崎君 第二季 / Jaku-Chara Tomozaki-kun 2nd Stage - 11 (B-Global 1920x1080 HEVC AAC MKV)[239.66 MB]".to_string(),
                season: 2,
                episode: 11,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(B-Global 1920x1080 HEVC AAC MKV)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240313/49b9c8dd833629d39e09a4e9568bde6b6a71a01b.torrent")
                    .content_len(251301728u64)
                    .pub_date("2024-03-13T20:31:07.116")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/f6d8f1b7131135c2c8b295aca18c64cb6405e2aa".to_string(),
                title: "公主殿下，「拷问」的时间到了".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 公主殿下，「拷问」的时间到了 / Himesama 'Goumon' no Jikan desu - 10 (CR 1920x1080 AVC AAC MKV)[1.37 GB]".to_string(),
                season: 1,
                episode: 10,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240312/f6d8f1b7131135c2c8b295aca18c64cb6405e2aa.torrent")
                    .content_len(1471026304u64)
                    .pub_date("2024-03-12T00:31:33.72")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/da075c8a8e0b9f71e130b978fb94e4def0745b30".to_string(),
                title: "我内心的糟糕念头".to_string(),
                episode_title: "".to_string(),
                description: "[喵萌奶茶屋&LoliHouse] 我内心的糟糕念头 / Boku no Kokoro no Yabai Yatsu - 22 [WebRip 1080p HEVC-10bit AAC][简繁日内封字幕][273.57 MB]".to_string(),
                season: 1,
                episode: 22,
                fansub: "[喵萌奶茶屋&LoliHouse]".to_string(),
                media_info: "[WebRip 1080p HEVC-10bit AAC][简繁日内封字幕]".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240310/da075c8a8e0b9f71e130b978fb94e4def0745b30.torrent")
                    .content_len(286858944u64)
                    .pub_date("2024-03-10T20:46:52.314")
                    .build()
                    .unwrap(),
            },
            RssSubscriptionItem {
                url: "https://mikanani.me/Home/Episode/6f9bb9e56663194eb68a0811890751d1e66f6fbd".to_string(),
                title: "我独自升级".to_string(),
                episode_title: "".to_string(),
                description: "[GJ.Y] 我独自升级 / Ore dake Level Up na Ken - 09 (CR 1920x1080 AVC AAC MKV)[1.37 GB]".to_string(),
                season: 1,
                episode: 9,
                fansub: "[GJ.Y]".to_string(),
                media_info: "(CR 1920x1080 AVC AAC MKV)".to_string(),
                torrent: TorrentMetaBuilder::default()
                    .url("https://mikanani.me/Download/20240310/6f9bb9e56663194eb68a0811890751d1e66f6fbd.torrent")
                    .content_len(1471026304u64)
                    .pub_date("2024-03-10T01:31:34.279")
                    .build()
                    .unwrap(),
            },
        ];
        res.items.iter().zip(expect.iter()).for_each(|(a, b)| {
            assert_eq!(a, b);
        });
    }
}
