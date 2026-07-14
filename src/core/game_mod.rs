use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum ModSource {
    ModDb {
        mod_id: i64,
        release_id: i64,
        logo: Option<PathBuf>,
        name: String,
        description: String,
        version: semver::Version,
    },
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GameMod {
    pub source: ModSource,
    //Mod zip file
    pub file: PathBuf,
}

impl GameMod {
    pub fn moddb(
        mod_id: i64,
        release_id: i64,
        install_path: PathBuf,
        logo: Option<PathBuf>,
        name: String,
        description: String,
        version: semver::Version,
    ) -> Self {
        Self {
            source: ModSource::ModDb {
                mod_id,
                release_id,
                logo,
                name,
                description,
                version,
            },
            file: install_path,
        }
    }
}
