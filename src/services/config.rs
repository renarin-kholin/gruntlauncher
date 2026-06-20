use std::{
    env::home_dir,
    fs::{self, File},
    io::Write,
    path::{self, PathBuf},
};

use tracing::{debug, info, warn};

use crate::core::config::Config;

#[derive(thiserror::Error, Debug, Clone)]
pub enum LoadConfigError {
    #[error("Error while reading the config file.")]
    IOError(String),

    #[error("Error when trying to deserialize config file.")]
    DeserializeError(#[from] toml::de::Error),

    #[error("Error when trying to serialize config file.")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Could not load project directories.")]
    ProjectDirError,
}
impl From<std::io::Error> for LoadConfigError {
    fn from(value: std::io::Error) -> Self {
        LoadConfigError::IOError(value.to_string())
    }
}

pub fn load_config() -> Result<Config, LoadConfigError> {
    info!("Loading config");

    let proj_dir = directories::ProjectDirs::from("com", "renarin", "gruntlauncher")
        .ok_or(LoadConfigError::ProjectDirError)?;

    let config_dir = proj_dir.config_dir();
    fs::create_dir_all(config_dir)?;
    let config_path = config_dir.join("config.toml");

    match fs::read_to_string(&config_path) {
        Ok(text) => Ok(toml::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let instances_dir = proj_dir.data_dir().join("instances");
            let config = Config::new(instances_dir, vec![]);
            fs::write(&config_path, toml::to_string_pretty(&config)?)?;
            Ok(config)
        }
        Err(e) => Err(e.into()),
    }
}
