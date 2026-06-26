use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{assets::VSAPI_VERSIONS, core::version::GameVersion};

#[derive(Deserialize, Debug)]
struct VSAPIVersionURLsObject {
    cdn: String,
    #[expect(dead_code)]
    local: String,
}
#[derive(Deserialize, Debug)]
struct VSAPIVersionFileObject {
    filename: String,
    #[expect(dead_code)]
    filesize: String,
    #[expect(dead_code)]
    md5: String,
    urls: VSAPIVersionURLsObject,
}
#[derive(Deserialize, Debug)]
struct VSAPIVersion {
    #[cfg(target_os = "windows")]
    windows: VSAPIVersionFileObject,
    #[cfg(target_os = "linux")]
    linux: VSAPIVersionFileObject,
    #[cfg(target_os = "macos")]
    mac_x64: VSAPIVersionFileObject,
}

impl VSAPIVersion {
    pub fn file_object(self) -> VSAPIVersionFileObject {
        #[cfg(target_os = "windows")]
        return self.windows;
        #[cfg(target_os = "linux")]
        return self.linux;
        #[cfg(target_os = "macos")]
        return self.mac_x64;
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct GameVersionStore {
    pub gameversions: Vec<GameVersion>,
}

impl GameVersionStore {
    pub fn empty() -> Self {
        Self {
            gameversions: vec![],
        }
    }
}
#[derive(Clone, Debug, Error)]
pub enum VersionsError {
    #[error("There was an IO Error in the version service: {0}")]
    IOError(String),
    #[error("Could not load project directories.")]
    ProjectDirError,

    #[error("Error while serializing game versions.")]
    TomlSerError(#[from] toml::ser::Error),
}
impl From<std::io::Error> for VersionsError {
    fn from(value: std::io::Error) -> Self {
        VersionsError::IOError(value.to_string())
    }
}
pub async fn load_local_versions(
    installations_path: &Path,
) -> Result<Vec<GameVersion>, VersionsError> {
    fs::create_dir_all(installations_path)?;
    Ok(vec![])
}

async fn get_versions_from_api(versions_path: PathBuf) -> Result<Vec<GameVersion>, VersionsError> {
    let mut gameversions: Vec<GameVersion> = vec![];

    let response = reqwest::get(VSAPI_VERSIONS).await.unwrap();
    let mut versions_toml = File::create(versions_path)?;

    let parsed_response = response
        .json::<HashMap<String, VSAPIVersion>>()
        .await
        .unwrap_or(HashMap::new());

    for (key, item) in parsed_response {
        let version = semver::Version::from_str(&key).unwrap();
        let file_object = item.file_object();
        let gameversion = GameVersion::remote(version, file_object.filename, file_object.urls.cdn);
        gameversions.push(gameversion);
    }

    let gameversion_store = GameVersionStore {
        gameversions: gameversions.clone(),
    };
    gameversions.sort_by_key(|v| v.version.clone());
    gameversions.reverse();
    versions_toml.write_all((toml::to_string(&gameversion_store)?).as_bytes())?;
    Ok(gameversions)
}
pub async fn refresh_versions() -> Result<Vec<GameVersion>, VersionsError> {
    let proj_dir = directories::ProjectDirs::from("com", "renarin", "gruntlauncher")
        .ok_or(VersionsError::ProjectDirError)?;
    let cache_dir = proj_dir.cache_dir();

    get_versions_from_api(cache_dir.join("versions.toml")).await
}

pub async fn load_versions() -> Result<Vec<GameVersion>, VersionsError> {
    let proj_dir = directories::ProjectDirs::from("com", "renarin", "gruntlauncher")
        .ok_or(VersionsError::ProjectDirError)?;
    let cache_dir = proj_dir.cache_dir();
    fs::create_dir_all(cache_dir)?;

    let mut gameversions: Vec<GameVersion> = vec![];
    if let Ok(versions_toml) = fs::read_to_string(cache_dir.join("versions.toml")) {
        let gameversion_store =
            toml::from_str::<GameVersionStore>(&versions_toml).unwrap_or(GameVersionStore::empty());
        gameversions.extend(gameversion_store.gameversions);
    }
    if gameversions.is_empty() {
        gameversions.extend(get_versions_from_api(cache_dir.join("versions.toml")).await?);
    };

    Ok(gameversions)
}
