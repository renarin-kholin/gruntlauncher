use std::{fs, path::Path};

use iced::futures::io;
use thiserror::Error;
use uuid::Uuid;

use crate::core::instance::GruntInstance;

#[derive(Error, Debug, Clone)]
pub enum LoadInstancesError {
    #[error("Could not read instances directory: {0}")]
    IOError(String),
}

impl From<io::Error> for LoadInstancesError {
    fn from(value: io::Error) -> Self {
        LoadInstancesError::IOError(value.to_string())
    }
}
pub fn load_instances(instances_path: &Path) -> Result<Vec<GruntInstance>, LoadInstancesError> {
    fs::create_dir_all(instances_path)?;
    let dir = fs::read_dir(instances_path)?;
    let mut instances = vec![];
    for entry in dir {
        let entry = entry?;
        if let Ok(filename) = entry.file_name().into_string() {
            instances.push(GruntInstance {
                name: filename,
                id: Uuid::new_v4(),
                mods: vec![],
            });
        } else {
            continue;
        }
    }

    Ok(instances)
}
