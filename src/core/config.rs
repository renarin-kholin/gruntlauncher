use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    core::account::Account,
    paths::{self},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub instances_folder: PathBuf,
    pub installations_folder: PathBuf,
    pub accounts: Vec<Account>,
}

impl Config {
    pub fn new(
        instances_folder: PathBuf,
        installations_folder: PathBuf,
        accounts: Vec<Account>,
    ) -> Self {
        Config {
            instances_folder,
            installations_folder,
            accounts,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        let project_dirs = paths::dirs().expect("Could not fetch default paths.");
        let data_dir = project_dirs.data_dir();
        Self::new(
            data_dir.join("instances"),
            data_dir.join("installations"),
            vec![],
        )
    }
}
