use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameVersion {
    pub version: semver::Version,
    pub filename: String,
    pub url: String,
}

pub enum VersionCatalog {
    NotLoaded,
    Loading,
    Loaded { versions: Vec<GameVersion> },
    Failed,
}
impl VersionCatalog {
    pub fn loading(&mut self) {
        *self = Self::Loading;
    }

    pub fn load(&mut self, gameversions: Vec<GameVersion>) {
        *self = Self::Loaded {
            versions: gameversions,
        };
    }

    pub fn failed(&mut self) {
        *self = Self::Failed;
    }
}
