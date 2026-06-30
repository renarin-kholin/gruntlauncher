use std::{
    fs::{self},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, error, info};

use crate::core::{instance::GruntInstance, version::GameVersionSource};

#[derive(Error, Debug, Clone)]
pub enum InstancesError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),
    #[error("error during serializing instance: {0}")]
    TomlSerError(#[from] toml::ser::Error),
    #[error("error during deserializing instance: {0}")]
    TomlDeError(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, InstancesError>;

impl From<std::io::Error> for InstancesError {
    fn from(value: std::io::Error) -> Self {
        InstancesError::Io(Arc::new(value))
    }
}
pub fn load_instances(instances_path: &Path) -> Result<Vec<GruntInstance>> {
    fs::create_dir_all(instances_path)?;
    let dir = fs::read_dir(instances_path)?;
    let mut instances = vec![];
    for entry in dir {
        let entry = entry?;
        debug!("{:?}", entry);
        if let Ok(instance_config) = fs::read_to_string(entry.path().join("instance.toml")) {
            match toml::from_str(&instance_config) {
                Ok(instance) => {
                    info!("Loaded instance config: {:?}", instance);
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

pub fn add_instance(instance: GruntInstance, instances_path: &Path) -> Result<GruntInstance> {
    fs::create_dir_all(instances_path)?;
    let instance_path = instances_path.join(instance.id.to_string());
    fs::create_dir(&instance_path)?;
    let mut instance_config = fs::File::create_new(instance_path.join("instance.toml"))?;
    instance_config.write_all((toml::to_string(&instance)?).as_bytes())?;

    Ok(instance)
}

pub async fn launch_instance(instance: GruntInstance, instances_path: PathBuf) -> Result<()> {
    if let GameVersionSource::Local(game) = instance.version.source {
        let data_path = instances_path.join(instance.id.to_string());
        let mods_path = data_path.join("Mods");
        #[cfg(not(target_os = "windows"))]
        let run_path = game.path.join("run.sh");
        #[cfg(target_os = "windows")]
        let run_path = game.path.join("Vintagestory");
        debug!("{:?}", run_path);
        debug!("{:?}", data_path);
        debug!("{:?}", mods_path);
        debug!(".{}", run_path.display());
        tokio::fs::create_dir_all(mods_path.clone()).await?;
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
