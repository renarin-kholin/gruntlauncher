use std::{path::PathBuf, sync::Arc};

use sipper::StreamExt;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReadDirStream;
use tracing::{error, info};
use uuid::Uuid;

use crate::core::instance::GruntInstance;

mod launch;

pub use launch::{launch_instance, patch_account_settings as patch_client_settings};

#[derive(Error, Debug, Clone)]
pub enum InstancesError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("error during serializing instance: {0}")]
    TomlSerError(#[from] toml::ser::Error),

    #[error("error during deserializing instance: {0}")]
    TomlDeError(#[from] toml::de::Error),

    #[error("Error during patching clientsettings: {0}")]
    ClientSettingsError(String),

    #[error(transparent)]
    SerdeJson(Arc<serde_json::Error>),

    #[error("Error while editing instance")]
    EditError,

    #[error("The game version must be installed before launching this instance.")]
    VersionNotInstalled,

    #[error("Could not launch {executable}: {reason}")]
    LaunchError { executable: PathBuf, reason: String },

    #[error("Game exited with {status}. See {log_path} for details.")]
    GameExited { status: String, log_path: PathBuf },
}

pub type Result<T> = std::result::Result<T, InstancesError>;

impl From<std::io::Error> for InstancesError {
    fn from(value: std::io::Error) -> Self {
        InstancesError::Io(Arc::new(value))
    }
}
impl From<serde_json::Error> for InstancesError {
    fn from(value: serde_json::Error) -> Self {
        InstancesError::SerdeJson(Arc::new(value))
    }
}

pub async fn delete_instance(instance: Uuid, instances_path: PathBuf) -> Result<()> {
    tokio::fs::create_dir_all(&instances_path).await?;
    let instance_path = instances_path.join(instance.to_string());
    tokio::fs::remove_dir_all(&instance_path).await?;
    Ok(())
}

pub async fn load_instances(instances_path: PathBuf) -> Result<Vec<GruntInstance>> {
    tokio::fs::create_dir_all(&instances_path).await?;
    let dir = tokio::fs::read_dir(instances_path).await?;
    let mut dir = ReadDirStream::new(dir);
    let mut instances = vec![];
    while let Some(entry) = dir.next().await {
        let entry = entry?;
        if let Ok(instance_config) =
            tokio::fs::read_to_string(entry.path().join("instance.toml")).await
        {
            match toml::from_str::<GruntInstance>(&instance_config) {
                Ok(instance) => {
                    info!(instance_name = instance.name, "Loaded instance config");
                    instances.push(instance)
                }
                Err(error) => {
                    error!(%error, "Error trying to parse instance config");
                }
            }
        }
    }
    Ok(instances)
}

pub async fn add_instance(
    instance: GruntInstance,
    instances_path: PathBuf,
) -> Result<GruntInstance> {
    let instance_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(&instance_path).await?;
    let mut instance_config =
        tokio::fs::File::create_new(instance_path.join("instance.toml")).await?;
    instance_config
        .write_all((toml::to_string(&instance)?).as_bytes())
        .await?;

    Ok(instance)
}

pub async fn save_instance(instance: GruntInstance, instances_path: PathBuf) -> Result<()> {
    let instance_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(&instance_path).await?;
    let mut instance_config = tokio::fs::File::create(instance_path.join("instance.toml")).await?;
    instance_config
        .write_all((toml::to_string(&instance)?).as_bytes())
        .await?;
    Ok(())
}
