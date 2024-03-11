use derive_builder::Builder;
use log::debug;
use std::path::{Path, PathBuf};

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

impl BangumiInfo {
    pub fn folder_name(&self) -> String {
        String::from(format!("{}", self.show_name))
    }

    pub fn sub_folder_name(&self) -> String {
        String::from(format!("Season {:02}", self.season))
    }

    pub fn file_name(&self, prefix: &str) -> String {
        assert!(prefix.starts_with('.'), "prefix must start with a dot");

        let mut file_name = String::new();

        file_name.push_str(&self.show_name.clone());
        file_name.push_str(&format!(" S{:02}E{:02}", self.season, self.episode));

        if let Some(ref episode_name) = self.episode_name {
            file_name.push_str(&format!(" {}", episode_name));
        }

        if let Some(ref display_name) = self.display_name {
            file_name.push_str(&format!(" {}", display_name));
        }

        file_name.push_str(prefix);

        file_name
    }

    pub fn gen_path(&self, prefix: &str) -> PathBuf {
        PathBuf::new()
            .join(self.folder_name())
            .join(self.sub_folder_name())
            .join(self.file_name(prefix))
    }
}

/// Link the file to the correct location.
/// e.g., if the `src_path` is `/download/The Big Bang Theory S01E01.mkv`,
/// and the `dst_folder` is `/media/TV`,
/// it should be linked to `/media/TV/The Big Bang Theory/Season 01/The Big Bang Theory S01E01.mkv`.
pub fn rename(info: &BangumiInfo, src_path: &Path, dst_folder: &Path) -> anyhow::Result<()> {
    if !src_path.exists() {
        return Err(anyhow::Error::msg("File does not exist"));
    }

    // TODO: support folder
    if !src_path.is_file() {
        return Err(anyhow::Error::msg("Unsupported file type"));
    }

    let extension = src_path
        .extension()
        .ok_or(anyhow::Error::msg(format!("File {} has no extension", &src_path.display())))?;

    let dst_path = dst_folder.join(info.gen_path(extension.to_str().unwrap()));

    link(src_path, &dst_path)?;

    Ok(())
}

pub fn link(src_path: &Path, dst_path: &Path) -> anyhow::Result<()> {
    if !src_path.is_file() {
        return Err(anyhow::Error::msg("Only file type can be linked"));
    }

    if dst_path.exists() {
        debug!("[renamer] File {} already linked", dst_path.display());
        return Ok(());
    }

    std::fs::hard_link(src_path, dst_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_name() {
        let bangumi_info = BangumiInfo {
            show_name: String::from("The Big Bang Theory"),
            season: 1,
            episode: 1,
            ..Default::default()
        };

        assert_eq!(bangumi_info.file_name(".mkv"), "The Big Bang Theory S01E01.mkv");
    }

    #[test]
    fn test_file_name_with_episode_name() {
        let bangumi_info = BangumiInfo {
            show_name: String::from("The Big Bang Theory"),
            season: 1,
            episode: 1,
            episode_name: Some(String::from("Pilot")),
            ..Default::default()
        };

        assert_eq!(bangumi_info.file_name(".mkv"), "The Big Bang Theory S01E01 Pilot.mkv");
    }

    #[test]
    fn test_file_name_with_display_name() {
        let bangumi_info = BangumiInfo {
            show_name: String::from("The Big Bang Theory"),
            season: 1,
            episode: 1,
            display_name: Some(String::from("720p")),
            ..Default::default()
        };

        assert_eq!(bangumi_info.file_name(".mkv"), "The Big Bang Theory S01E01 720p.mkv");
    }
}
