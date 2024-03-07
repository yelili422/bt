use derive_builder::Builder;

#[derive(Default, Builder, Debug, PartialEq, Eq)]
#[builder(setter(into))]
pub struct BangumiInfo {
    pub show_name: String,
    pub episode_name: Option<String>,
    pub display_name: Option<String>,
    pub season: u64,
    pub episode: u64,
    pub category: Option<String>,
}
