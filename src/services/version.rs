use std::{collections::HashMap, str::FromStr};

use serde::Deserialize;
use serde_json::{Map, Value};
use tracing::warn;

use crate::{
    assets::VSAPI_VERSIONS,
    core::version::{GameVersion, VersionCatalog},
};

#[derive(Deserialize, Debug)]
struct VSAPIVersionURLsObject {
    cdn: String,
    local: String,
}
#[derive(Deserialize, Debug)]
struct VSAPIVersionFileObject {
    filename: String,
    filesize: String,
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

#[derive(Clone, Debug)]
pub enum VersionsError {}

pub async fn load_versions() -> Result<Vec<GameVersion>, VersionsError> {
    let response = reqwest::get(VSAPI_VERSIONS).await.unwrap();
    let parsed_response = response
        .json::<HashMap<String, VSAPIVersion>>()
        .await
        .unwrap();
    let mut gameversions: Vec<GameVersion> = vec![];

    for (key, item) in parsed_response {
        let version = semver::Version::from_str(&key).unwrap();
        let file_object = item.file_object();
        let gameversion = GameVersion {
            version,
            filename: file_object.filename,
            url: file_object.urls.cdn,
        };
        gameversions.push(gameversion);
    }

    gameversions.sort_by_key(|v| v.version.clone());
    gameversions.reverse();

    Ok(gameversions)
}
