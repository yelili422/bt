use derive_builder::Builder;

#[derive(Default, Builder, Debug, PartialEq, Eq)]
#[builder(setter(into))]
pub struct TvRules {
    pub show_name: String,
    pub episode_name: String,
    pub display_name: String,
    pub season: u64,
    pub episode: u64,
    pub category: String,
}
