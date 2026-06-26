use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Account {
    email: String,
    //TODO: Find what to store for account
}

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
