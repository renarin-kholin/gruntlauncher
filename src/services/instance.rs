use std::{path::PathBuf, sync::Arc};

use serde_json::{json, Value};
use sipper::StreamExt;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream::wrappers::ReadDirStream;
use tracing::{debug, error, info};

use crate::{
    core::{account::Account, instance::GruntInstance, version::GameVersionSource},
    services::instance::InstancesError::ClientSettingsError,
};

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
                    info!("Loaded instance config: {}", instance.name);
                    instances.push(instance)
                }
                Err(e) => {
                    error!("Error trying to parse instance config: {}", e);
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
    tokio::fs::create_dir_all(&instances_path).await?;
    let instance_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(&instance_path).await?;
    let mut instance_config =
        tokio::fs::File::create_new(instance_path.join("instance.toml")).await?;
    instance_config
        .write_all((toml::to_string(&instance)?).as_bytes())
        .await?;

    Ok(instance)
}

pub async fn patch_client_settings(instance_path: PathBuf, account: Account) -> Result<()> {
    let clientsettings_file = instance_path.join("clientsettings.json");
    let root_text = match tokio::fs::read_to_string(&clientsettings_file).await {
        Ok(text) => text,
        Err(e) if e.kind() == tokio::io::ErrorKind::NotFound => "{}".to_string(),
        Err(e) => return Err(e.into()),
    };
    let mut root: Value = serde_json::from_str(&root_text)?;
    let ss = root
        .as_object_mut()
        .ok_or_else(|| ClientSettingsError("clientsettings.json root is not an object".into()))?
        .entry("stringsettings")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| ClientSettingsError("stringsettings exists but is not an object".into()))?;
    ss.insert("sessionkey".to_string(), json!(account.sessionkey));
    ss.insert(
        "sessionsignature".to_string(),
        json!(account.sessionsignature),
    );
    ss.insert("playeruid".to_string(), json!(account.uid));
    ss.insert("playername".to_string(), json!(account.username));
    tokio::fs::write(&clientsettings_file, serde_json::to_string_pretty(&root)?).await?;
    Ok(())
}
pub async fn launch_instance(
    instance: GruntInstance,
    instances_path: PathBuf,
    account: Option<Account>,
) -> Result<()> {
    if let GameVersionSource::Local(game) = instance.version.source {
        let data_path = instances_path.join(instance.id.to_string());
        let mods_path = data_path.join("Mods");
        #[cfg(not(target_os = "windows"))]
        let run_path = game.path.join("run.sh");
        #[cfg(target_os = "windows")]
        let run_path = game.path.join("Vintagestory.exe");
        debug!("{:?}", run_path);
        debug!("{:?}", data_path);
        debug!("{:?}", mods_path);
        debug!(".{}", run_path.display());
        tokio::fs::create_dir_all(mods_path.clone()).await?;
        if let Some(account) = account {
            match patch_client_settings(data_path.clone(), account).await {
                Ok(()) => {
                    info!("Successfully patched clientsettings.json")
                }
                Err(e) => {
                    error!("{e}");
                }
            }
        }
        Command::new(run_path)
            .arg("--dataPath")
            .arg(data_path)
            .arg("--addModPath")
            .arg(mods_path)
            .status()
            .await?;
    } else {
        error!("Could not launch the game.")
    }
    Ok(())
}
