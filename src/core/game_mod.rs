use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ModSource {
    ModDb {
        mod_id: i64,
        release_id: i64,
        logo: Option<PathBuf>,
        name: String,
        description: String,
    },
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    ) -> Self {
        Self {
            source: ModSource::ModDb {
                mod_id,
                release_id,
                logo,
                name,
                description,
            },
            file: install_path,
        }
    }
}
