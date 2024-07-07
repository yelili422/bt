use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use typed_builder::TypedBuilder;

#[derive(Default, TypedBuilder, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BangumiInfo {
    pub show_name: String,
    #[builder(default)]
    pub episode_name: Option<String>,
    #[builder(default)]
    pub display_name: Option<String>,
    pub season: u64,
    pub episode: u64,
    #[builder(default)]
    pub category: Option<String>,
}

impl BangumiInfo {
    pub fn folder_name(&self) -> String {
        String::from(format!("{}", self.show_name))
    }

    pub fn sub_folder_name(&self) -> String {
        String::from(format!("Season {}", self.season))
    }

    pub fn file_name(&self, extension: &str) -> String {
        assert!(extension.starts_with('.'), "extension must start with a dot");

        let mut file_name = self.file_name_without_extension();

        file_name.push_str(extension);

        file_name
    }

    pub fn file_name_without_extension(&self) -> String {
        let mut file_name = String::new();

        file_name.push_str(&self.show_name.clone());
        file_name.push_str(&format!(" S{:02}E{:02}", self.season, self.episode));

        if let Some(ref episode_name) = self.episode_name {
            if !episode_name.is_empty() {
                file_name.push_str(&format!(" {}", episode_name));
            }
        }

        if let Some(ref display_name) = self.display_name {
            if !display_name.is_empty() {
                file_name.push_str(&format!(" {}", display_name));
            }
        }

        file_name
    }

    pub fn gen_path(&self, extension: &str) -> PathBuf {
        PathBuf::new()
            .join(self.folder_name())
            .join(self.sub_folder_name())
            .join(self.file_name(&format!(".{}", extension)))
    }
}

/// Link the file to the correct location.
/// e.g., if the `src_path` is `/download/Sousou no Frieren S01E01.mkv`,
/// and the `dst_folder` is `/media/TV`,
/// it should be linked to `/media/TV/Sousou no Frieren/Season 1/Sousou no Frieren S01E01.mkv`.
///
/// Note: `src_path` means the original path may be a file or a folder.
pub fn rename(info: &BangumiInfo, src_path: &Path, dst_folder: &Path) -> anyhow::Result<()> {
    debug!("[rename] Renaming {} to {}", src_path.display(), info.gen_path("mkv").display());

    if !src_path.exists() {
        return Err(anyhow::Error::msg(format!("File {} not exists", src_path.display())));
    }

    if src_path.is_file() {
        let extension = src_path
            .extension()
            .ok_or(anyhow::Error::msg(format!("File {} has no extension", &src_path.display())))?;

        let dst_path = dst_folder.join(info.gen_path(extension.to_str().unwrap()));

        link(src_path, &dst_path)?;
    } else if src_path.is_dir() {
        for entry in std::fs::read_dir(src_path)? {
            let entry = entry?;

            if entry.file_type()?.is_dir() {
                continue;
            }

            let entry_path = entry.path();
            let extension = entry_path.extension().ok_or(anyhow::Error::msg(format!(
                "File {} has no extension",
                &entry_path.display()
            )))?;

            let dst_path = dst_folder.join(info.gen_path(extension.to_str().unwrap()));

            link(&entry_path, &dst_path)?;
        }
    }

    Ok(())
}

pub fn link(src_path: &Path, dst_path: &Path) -> anyhow::Result<()> {
    info!("[rename] Linking {} to {}", src_path.display(), dst_path.display());
    if !src_path.is_file() {
        return Err(anyhow::Error::msg("Only file type can be linked"));
    }

    let dst_parent = dst_path.parent().unwrap();
    if !dst_parent.exists() {
        info!("[rename] Target folder {} not exists, creating", dst_parent.display());
        std::fs::create_dir_all(dst_parent)?;
    }

    if dst_path.exists() {
        info!("[rename] File {} already linked", dst_path.display());
        return Ok(());
    }

    std::fs::hard_link(src_path, dst_path)?;

    Ok(())
}

/// Replace the path according to the rule.
/// e.g. `path` is "/download/Sousou no Frieren S01E01.mkv",
/// and `replace_rule` is "/download:/tmp",
/// it should return "/tmp/Sousou no Frieren S01E01.mkv"
pub fn replace_path(path: PathBuf, replace_rule: &str) -> PathBuf {
    if replace_rule.is_empty() {
        return path;
    }
    let path = path.to_str().unwrap();

    let mut replace_rule = replace_rule.split(':');
    let src = replace_rule.next().unwrap();
    let dst = replace_rule.next().unwrap();

    PathBuf::from(path.replace(src, dst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_path() {
        let bangumi_infos = vec![
            BangumiInfo {
                show_name: String::from("Sousou no Frieren"),
                season: 1,
                episode: 12,
                ..Default::default()
            },
            BangumiInfo {
                show_name: String::from("Sousou no Frieren"),
                episode_name: Some(String::from("冒险的终点")),
                display_name: None,
                season: 1,
                episode: 1,
                category: None,
            },
        ];

        let res_paths = vec![
            PathBuf::from("Sousou no Frieren/Season 1/Sousou no Frieren S01E12.mkv"),
            PathBuf::from("Sousou no Frieren/Season 1/Sousou no Frieren S01E01 冒险的终点.mkv"),
        ];

        for (info, res_path) in bangumi_infos.iter().zip(res_paths.iter()) {
            assert_eq!(info.gen_path("mkv"), *res_path);
        }
    }

    #[test]
    fn test_rename() {
        let src_path = Path::new("/tmp/Sousou no Frieren S01E00.mkv");

        std::fs::write(src_path, "test").unwrap();

        let dst_folder = Path::new("/tmp/TV");
        let bangumi_info = BangumiInfo {
            show_name: String::from("Sousou no Frieren"),
            season: 1,
            episode: 12,
            ..Default::default()
        };

        rename(&bangumi_info, src_path, dst_folder).unwrap();

        let dst_path = dst_folder.join("Sousou no Frieren/Season 1/Sousou no Frieren S01E12.mkv");
        let content = std::fs::read_to_string(dst_path).unwrap();
        assert_eq!(content, "test");
    }

    #[test]
    fn test_rename_dir() {
        let src_dir = Path::new("/tmp/迷宫饭");
        if !src_dir.exists() {
            std::fs::create_dir(src_dir).unwrap();
        }

        std::fs::write(&src_dir.join("迷宫饭[1].mkv"), "mkv_content").unwrap();
        std::fs::write(&src_dir.join("迷宫饭[1].ass"), "ass_content").unwrap();

        let dst_folder = Path::new("/tmp/TV");
        let bangumi_info = BangumiInfo {
            show_name: String::from("迷宫饭"),
            season: 1,
            episode: 1,
            ..Default::default()
        };

        rename(&bangumi_info, src_dir, dst_folder).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst_folder.join("迷宫饭/Season 1/迷宫饭 S01E01.mkv")).unwrap(),
            "mkv_content"
        );
        assert_eq!(
            std::fs::read_to_string(dst_folder.join("迷宫饭/Season 1/迷宫饭 S01E01.ass")).unwrap(),
            "ass_content"
        );
    }
}
