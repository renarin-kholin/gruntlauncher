use std::{fs, io::Write, path::Path};

use iced::futures::io;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::core::instance::GruntInstance;

#[derive(Error, Debug, Clone)]
pub enum InstancesError {
    #[error("Could not read instances directory: {0}")]
    IOError(String),
    #[error("Error during serializing instance: {0}")]
    TomlSerError(#[from] toml::ser::Error),
    #[error("Error during deserializing instance: {0}")]
    TomlDeError(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, InstancesError>;

impl From<io::Error> for InstancesError {
    fn from(value: io::Error) -> Self {
        InstancesError::IOError(value.to_string())
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
