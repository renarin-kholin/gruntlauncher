use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::{self};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    LightHighContrast,
    DarkHighContrast,
    #[default]
    #[serde(other)]
    Default,
}

impl ThemeChoice {
    pub const ALL: [Self; 3] = [
        Self::Default,
        Self::LightHighContrast,
        Self::DarkHighContrast,
    ];
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Default => "Default",
            Self::LightHighContrast => "Light High Contrast",
            Self::DarkHighContrast => "Dark High Contrast",
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub instances_folder: PathBuf,
    pub installations_folder: PathBuf,
    #[serde(default)]
    pub theme: ThemeChoice,
}

impl Config {
    pub fn new(instances_folder: PathBuf, installations_folder: PathBuf) -> Self {
        Config {
            instances_folder,
            installations_folder,
            theme: ThemeChoice::default(),
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        let project_dirs = paths::dirs().expect("Could not fetch default paths.");
        let data_dir = project_dirs.data_dir();
        Self::new(data_dir.join("instances"), data_dir.join("installations"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_configs_and_unknown_themes_keep_paths_and_use_default() {
        let original = "instances_folder = 'instances'\ninstallations_folder = 'installations'\n";
        for source in [
            original.to_string(),
            format!("{original}theme = 'unavailable'\n"),
        ] {
            let config: Config = toml::from_str(&source).unwrap();
            assert_eq!(config.theme, ThemeChoice::Default);
            assert_eq!(config.instances_folder, PathBuf::from("instances"));
            assert_eq!(config.installations_folder, PathBuf::from("installations"));
        }
    }

    #[test]
    fn theme_ids_round_trip_in_config() {
        for (theme, id) in [
            (ThemeChoice::Default, "default"),
            (ThemeChoice::LightHighContrast, "light-high-contrast"),
            (ThemeChoice::DarkHighContrast, "dark-high-contrast"),
        ] {
            let mut config = Config::new("instances".into(), "installations".into());
            config.theme = theme;
            let source = toml::to_string_pretty(&config).unwrap();
            assert!(source.contains(&format!("theme = \"{id}\"")));
            let loaded: Config = toml::from_str(&source).unwrap();
            assert_eq!(loaded.theme, theme);
            assert_eq!(loaded.instances_folder, config.instances_folder);
            assert_eq!(loaded.installations_folder, config.installations_folder);
        }
    }
}
