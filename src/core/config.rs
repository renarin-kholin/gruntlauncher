use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::{self};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub instances_folder: PathBuf,
    pub installations_folder: PathBuf,
}

impl Config {
    pub fn new(instances_folder: PathBuf, installations_folder: PathBuf) -> Self {
        Config {
            instances_folder,
            installations_folder,
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
