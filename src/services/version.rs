use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

use iced::futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream::wrappers::ReadDirStream;
use tracing::{debug, error, warn};

use crate::{
    assets::VSAPI_VERSIONS,
    core::version::{GameVersion, GameVersionSource, merge_versions},
    paths::{self, ProjectDirError},
};

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
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error(transparent)]
    ProjectDir(#[from] ProjectDirError),

    #[error("error while serializing game versions: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("received response had no content length specified")]
    NoContentLength,

    #[error("request to the server failed: {0}")]
    Reqwest(Arc<reqwest::Error>),

    #[error("Failed downloading archive")]
    DownloadError,
}
impl From<std::io::Error> for VersionsError {
    fn from(value: std::io::Error) -> Self {
        VersionsError::Io(Arc::new(value))
    }
}
impl From<reqwest::Error> for VersionsError {
    fn from(value: reqwest::Error) -> Self {
        VersionsError::Reqwest(Arc::new(value))
    }
}
pub async fn load_local_versions(
    installations_path: PathBuf,
) -> Result<Vec<GameVersion>, VersionsError> {
    tokio::fs::create_dir_all(&installations_path).await?;
    let versions_dir = tokio::fs::read_dir(installations_path).await?;
    let mut versions_dir = ReadDirStream::new(versions_dir);
    let mut gameversions = vec![];
    while let Some(version) = versions_dir.next().await {
        let version = version?;
        match semver::Version::from_str(&format!("{}", version.file_name().as_os_str().display())) {
            Ok(parsed_version) => {
                let gameversion = GameVersion::local(parsed_version, &version.path());

                gameversions.push(gameversion);
                debug!("Found game version: {}", version.file_name().display());
            }
            Err(e) => {
                error!("Failed to parse a folder in the installations directory: {e:?}");
            }
        }
    }

    Ok(gameversions)
}

fn sort_newest_first(versions: &mut [GameVersion]) {
    versions.sort_by(|a, b| b.version.cmp(&a.version));
}

async fn get_versions_from_api(
    versions_path: PathBuf,
    installations_path: PathBuf,
) -> Result<Vec<GameVersion>, VersionsError> {
    let response = reqwest::get(VSAPI_VERSIONS).await?;

    let parsed_response = response.json::<HashMap<String, VSAPIVersion>>().await?;
    let gameversions: Vec<GameVersion> = parsed_response
        .into_iter()
        .filter_map(|(key, item)| match semver::Version::from_str(&key) {
            Ok(version) => {
                let file_object = item.file_object();
                Some(GameVersion::remote(
                    version,
                    file_object.filename,
                    file_object.urls.cdn,
                ))
            }
            Err(e) => {
                warn!("Skipping unparseable game version: {key:?}: {e}");
                None
            }
        })
        .collect();

    let mut gameversions =
        merge_versions(load_local_versions(installations_path).await?, gameversions);
    sort_newest_first(&mut gameversions);

    let gameversion_store = GameVersionStore {
        gameversions: gameversions.clone(),
    };

    let mut versions_toml = tokio::fs::File::create(versions_path).await?;
    versions_toml
        .write_all((toml::to_string(&gameversion_store)?).as_bytes())
        .await?;
    Ok(gameversions)
}
pub async fn refresh_versions(
    installations_path: PathBuf,
) -> Result<Vec<GameVersion>, VersionsError> {
    let cache_dir = paths::cache_dir()?;
    tokio::fs::create_dir_all(&cache_dir).await?;

    get_versions_from_api(cache_dir.join("versions.toml"), installations_path).await
}

pub async fn load_versions(installations_path: PathBuf) -> Result<Vec<GameVersion>, VersionsError> {
    let cache_dir = paths::cache_dir()?;
    tokio::fs::create_dir_all(&cache_dir).await?;

    let mut gameversions: Vec<GameVersion> = vec![];
    if let Ok(versions_toml) = tokio::fs::read_to_string(cache_dir.join("versions.toml")).await {
        let gameversion_store =
            toml::from_str::<GameVersionStore>(&versions_toml).unwrap_or(GameVersionStore::empty());
        gameversions.extend(gameversion_store.gameversions);
    }
    if gameversions.is_empty() {
        gameversions.extend(
            get_versions_from_api(cache_dir.join("versions.toml"), installations_path).await?,
        );
    };

    Ok(gameversions)
}

#[derive(Debug, Clone)]
pub enum InstallProgress {
    NotStarted,
    Downloading { downloaded: u64, total: u64 },
    Verifying,
    Installing,
    Done,
    Failed(VersionsError),
}
pub async fn download_version(
    gameversion: GameVersion,
    progress: &mut sipper::Sender<InstallProgress>,
) -> Result<Option<PathBuf>, VersionsError> {
    if let GameVersionSource::Remote(remote_mod) = &gameversion.source {
        let cache_dir = paths::cache_dir()?;
        let temp_download_path = cache_dir.join(gameversion.version.to_string());
        tokio::fs::create_dir_all(&temp_download_path).await?;

        let temp_file_path = temp_download_path.join(&remote_mod.filename);
        let mut temp_file = tokio::fs::File::create(&temp_file_path).await?;
        let response = reqwest::get(&remote_mod.url).await?;
        let total = response
            .content_length()
            .ok_or(VersionsError::NoContentLength)?;
        let _ = progress
            .send(InstallProgress::Downloading {
                downloaded: 0,
                total,
            })
            .await;
        let mut byte_stream = response.bytes_stream();
        let mut downloaded = 0;
        while let Some(next_bytes) = byte_stream.next().await {
            let bytes = next_bytes?;
            tokio::io::AsyncWriteExt::write_all(&mut temp_file, &bytes).await?;
            downloaded += bytes.len();
            progress
                .send(InstallProgress::Downloading {
                    downloaded: downloaded as u64,
                    total,
                })
                .await;
        }
        return Ok(Some(temp_file_path));
    }
    Ok(None)
}
#[cfg(not(target_os = "windows"))]
pub async fn extract_archive(
    gameversion: GameVersion,
    archive_path: PathBuf,
    versions_path: PathBuf,
    progress: &mut sipper::Sender<InstallProgress>,
) -> Result<PathBuf, VersionsError> {
    let install_path = versions_path.join(gameversion.version.to_string());

    tokio::fs::create_dir_all(&install_path).await?;

    progress.send(InstallProgress::Installing).await;
    let mut extract_child = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path.as_os_str())
        .arg("-C")
        .arg(install_path.as_os_str())
        .arg("--strip-components=1")
        .spawn()?;
    extract_child.wait().await?;

    Ok(install_path)
}

#[cfg(target_os = "windows")]
pub async fn extract_archive(
    gameversion: GameVersion,
    archive_path: PathBuf,
    versions_path: PathBuf,
    progress: &mut sipper::Sender<InstallProgress>,
) -> Result<PathBuf, VersionsError> {
    let install_path = versions_path.join(gameversion.version.to_string());
    tokio::fs::create_dir_all(&install_path).await?;

    progress.send(InstallProgress::Installing).await;

    let mut install_child = Command::new(archive_path.as_os_str())
        .arg("/VERYSILENT")
        .arg("/SUPRESSMSGBOXES")
        .arg("/DIR=")
        .arg(install_path.as_os_str())
        .status()
        .await?;
    Ok(install_path)
}
