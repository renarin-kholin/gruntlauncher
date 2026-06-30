use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct LocalGameVersion {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct RemoteGameVersion {
    pub filename: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub enum GameVersionSource {
    Local(LocalGameVersion),
    Remote(RemoteGameVersion),
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct GameVersion {
    pub version: semver::Version,
    pub source: GameVersionSource,
}
impl GameVersion {
    pub fn remote(version: semver::Version, filename: String, url: String) -> Self {
        Self {
            version,
            source: GameVersionSource::Remote(RemoteGameVersion { filename, url }),
        }
    }
    pub fn local(version: semver::Version, path: &Path) -> Self {
        Self {
            version,
            source: GameVersionSource::Local(LocalGameVersion { path: path.into() }),
        }
    }
    pub fn to_local(&self, path: &Path) -> Self {
        Self {
            version: self.version.clone(),
            source: GameVersionSource::Local(LocalGameVersion { path: path.into() }),
        }
    }
}
pub fn merge_versions(local: Vec<GameVersion>, remote: Vec<GameVersion>) -> Vec<GameVersion> {
    let mut by_version: HashMap<semver::Version, GameVersion> = HashMap::new();

    for gv in remote {
        by_version.insert(gv.version.clone(), gv);
    }
    for gv in local {
        by_version.insert(gv.version.clone(), gv); // inserted second => local overwrites remote
    }

    let mut out: Vec<GameVersion> = by_version.into_values().collect();
    out.sort_by(|a, b| b.version.cmp(&a.version)); // newest first
    out
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
