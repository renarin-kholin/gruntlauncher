use std::fs::{self};
use std::sync::Arc;

use tracing::info;

use crate::core::config::Config;
use crate::paths::{self, ProjectDirError};

#[derive(thiserror::Error, Debug, Clone)]
pub enum LoadConfigError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("error when trying to deserialize config file: {0}")]
    DeserializeError(#[from] toml::de::Error),

    #[error("error when trying to serialize config file: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error(transparent)]
    ProjectDir(#[from] ProjectDirError),
}
impl From<std::io::Error> for LoadConfigError {
    fn from(value: std::io::Error) -> Self {
        LoadConfigError::Io(Arc::new(value))
    }
}

pub fn load_config() -> Result<Config, LoadConfigError> {
    info!("Loading config");

    let config_dir = paths::config_dir()?;
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");

    match fs::read_to_string(&config_path) {
        Ok(text) => Ok(toml::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            fs::write(&config_path, toml::to_string_pretty(&config)?)?;
            Ok(config)
        }
        Err(e) => Err(e.into()),
    }
}
