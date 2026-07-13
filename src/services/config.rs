use std::path::PathBuf;
use std::sync::Arc;

use rfd::AsyncFileDialog;
use tracing::info;

use crate::core::config::Config;
use crate::paths::{self, ProjectDirError};

#[derive(thiserror::Error, Debug, Clone)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("error when trying to deserialize config file: {0}")]
    DeserializeError(#[from] toml::de::Error),

    #[error("error when trying to serialize config file: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error(transparent)]
    ProjectDir(#[from] ProjectDirError),

    #[error("Could not pick a folder")]
    FolderSelectError,
}
impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::Io(Arc::new(value))
    }
}

pub async fn load_config() -> Result<Config, ConfigError> {
    info!("Loading config");

    let config_dir = paths::config_dir()?;
    tokio::fs::create_dir_all(&config_dir).await?;
    let config_path = config_dir.join("config.toml");

    match tokio::fs::read_to_string(&config_path).await {
        Ok(text) => Ok(toml::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            tokio::fs::write(&config_path, toml::to_string_pretty(&config)?).await?;
            Ok(config)
        }
        Err(e) => Err(e.into()),
    }
}
pub async fn save_config(config: Config) -> Result<(), ConfigError> {
    let config_dir = paths::config_dir()?;
    tokio::fs::create_dir_all(&config_dir).await?;
    let config_path = config_dir.join("config.toml");
    tokio::fs::write(&config_path, toml::to_string_pretty(&config)?).await?;
    Ok(())
}
pub async fn reset_config() -> Result<Config, ConfigError> {
    let config_dir = paths::config_dir()?;
    tokio::fs::create_dir_all(&config_dir).await?;
    let config_path = config_dir.join("config.toml");

    let config = Config::default();
    tokio::fs::write(&config_path, toml::to_string_pretty(&config)?).await?;
    Ok(config)
}
pub async fn pick_folder(initial_path: PathBuf) -> Result<PathBuf, ConfigError> {
    let folder = AsyncFileDialog::new()
        .set_directory(initial_path)
        .pick_folder()
        .await;
    folder
        .ok_or(ConfigError::FolderSelectError)
        .map(|f| f.path().into())
}
