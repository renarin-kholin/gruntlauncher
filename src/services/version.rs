use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
#[cfg(target_os = "windows")]
use std::{ffi::OsString, path::Path};

use iced::futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream::wrappers::ReadDirStream;
use tracing::{debug, error, info, warn};

use crate::{
    assets::VSAPI_VERSIONS,
    core::version::{GameVersion, GameVersionSource, merge_versions},
    paths::{self, ProjectDirError},
    services::{HTTP, game_mod::ModDownloadProgress},
};

#[cfg(target_os = "windows")]
use crate::assets::VSWINREGKEY;

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

    #[error("Version installer failed: {0}")]
    InstallerError(String),

    #[error("Failed to verify checksum")]
    VerificationError,

    #[error("Error while creating blocking task: {0}")]
    TokioJoinError(Arc<tokio::task::JoinError>),
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

impl From<tokio::task::JoinError> for VersionsError {
    fn from(value: tokio::task::JoinError) -> Self {
        VersionsError::TokioJoinError(Arc::new(value))
    }
}
pub type Result<T> = std::result::Result<T, VersionsError>;
pub async fn load_local_versions(installations_path: PathBuf) -> Result<Vec<GameVersion>> {
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
) -> Result<Vec<GameVersion>> {
    let response = HTTP.get(VSAPI_VERSIONS).send().await?;

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
                    file_object.md5,
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
pub async fn refresh_versions(installations_path: PathBuf) -> Result<Vec<GameVersion>> {
    let cache_dir = paths::cache_dir()?;
    tokio::fs::create_dir_all(&cache_dir).await?;

    get_versions_from_api(cache_dir.join("versions.toml"), installations_path).await
}

pub async fn load_versions(installations_path: PathBuf) -> Result<Vec<GameVersion>> {
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
pub enum InstallStatus {
    NotStarted,
    Downloading { downloaded: u64, total: u64 },
    Verifying,
    Installing,
    DownloadingMods(i64, ModDownloadProgress),
    Done,
    Failed(VersionsError),
}

pub async fn download_version(
    gameversion: GameVersion,
    progress: &mut sipper::Sender<InstallStatus>,
) -> Result<Option<PathBuf>> {
    if let GameVersionSource::Remote(remote_game) = &gameversion.source {
        let cache_dir = paths::cache_dir()?;
        let temp_download_path = cache_dir.join(gameversion.version.to_string());
        tokio::fs::create_dir_all(&temp_download_path).await?;

        let temp_file_path = temp_download_path.join(&remote_game.filename);
        let verified = if temp_file_path.exists() {
            //verify the package is not corrupted
            info!("Existing download found verifying.");
            progress.send(InstallStatus::Verifying).await;
            verify_download(remote_game.checksum.clone(), temp_file_path.clone()).await
        } else {
            false
        };
        debug!("Verification result: {verified}");
        if let Ok(mut temp_file) = tokio::fs::File::create(&temp_file_path).await
            && !verified
        {
            //The file already exsits otherwise
            let response = HTTP.get(&remote_game.url).send().await?;
            let total = response
                .content_length()
                .ok_or(VersionsError::NoContentLength)?;
            let _ = progress
                .send(InstallStatus::Downloading {
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
                    .send(InstallStatus::Downloading {
                        downloaded: downloaded as u64,
                        total,
                    })
                    .await;
            }
            progress.send(InstallStatus::Verifying).await;
            if verify_download(remote_game.checksum.clone(), temp_file_path.clone()).await {
                info!("Verified download");
            } else {
                return Err(VersionsError::VerificationError);
            }
        }
        return Ok(Some(temp_file_path));
    }
    Ok(None)
}
pub async fn verify_download(checksum: String, download_file: PathBuf) -> bool {
    let computed_hash = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&download_file)?;
        Ok::<String, VersionsError>(format!("{:x}", md5::compute(bytes)))
    })
    .await;
    match computed_hash {
        Ok(Ok(hash)) => hash == checksum,

        Err(e) => {
            error!("{e}");
            false
        }
        _ => false,
    }
}

pub async fn install_game(
    gameversion: GameVersion,
    archive_path: PathBuf,
    versions_path: PathBuf,
    progress: &mut sipper::Sender<InstallStatus>,
) -> Result<PathBuf> {
    let install_path = versions_path.join(gameversion.version.to_string());

    tokio::fs::create_dir_all(&install_path).await?;

    progress.send(InstallStatus::Installing).await;

    #[cfg(target_os = "windows")]
    return windows_install(
        gameversion,
        archive_path,
        versions_path,
        install_path,
        progress,
    )
    .await;
    #[cfg(not(target_os = "windows"))]
    return extract_archive(
        gameversion,
        archive_path,
        versions_path,
        install_path,
        progress,
    )
    .await;
}
#[cfg(not(target_os = "windows"))]
pub async fn extract_archive(
    _gameversion: GameVersion,
    archive_path: PathBuf,
    _versions_path: PathBuf,
    install_path: PathBuf,
    _progress: &mut sipper::Sender<InstallStatus>,
) -> Result<PathBuf> {
    let extract_child = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path.as_os_str())
        .arg("-C")
        .arg(install_path.as_os_str())
        .arg("--strip-components=1")
        .status();
    extract_child.await?;

    Ok(install_path)
}
#[cfg(target_os = "windows")]
enum RegistryStatus {
    Clean,
    TaintedWithGruntLauncher,
    Tainted,
}
#[cfg(target_os = "windows")]
async fn is_registry_tainted(install_path: &PathBuf) -> Result<RegistryStatus> {
    use RegistryStatus::*;
    if let Ok(vs_reg_key) = winreg::HKCU.open_subkey(VSWINREGKEY) {
        if let Ok(reg_install_location) = vs_reg_key.get_value::<String, &str>("InstallLocation") {
            if Path::new(&reg_install_location).starts_with(install_path) {
                return Ok(TaintedWithGruntLauncher);
            } else {
                return Ok(Tainted);
            }
        }
    } else {
        warn!("Could not read vintage story registry key");
    }
    Ok(Clean)
}
//Cleans registry subkeys created as a side effect of grunt launcher running the game installer
#[cfg(target_os = "windows")]
async fn clean_registry() {
    if winreg::HKCU.delete_subkey_all(VSWINREGKEY).is_err() {
        error!("Could not delete registry tainted subkeys.");
    }
}
#[cfg(target_os = "windows")]
async fn windows_install(
    _gameversion: GameVersion,
    archive_path: PathBuf,
    _versions_path: PathBuf,
    install_path: PathBuf,
    progress: &mut sipper::Sender<InstallStatus>,
) -> Result<PathBuf> {
    //Check if the user already has a vintage story installation that might interfere with the
    //launcher's installation
    let mut saved_reg_key: Option<Vec<(String, String)>> = None;
    match is_registry_tainted(&install_path).await? {
        RegistryStatus::Clean => {
            debug!("Registry is clean.")
        }
        RegistryStatus::Tainted => {
            debug!("Original installation found that was not created by gruntlauncher.");
            saved_reg_key = winreg::HKCU.open_subkey(VSWINREGKEY).ok().map(|key| {
                key.enum_values()
                    .filter_map(std::result::Result::ok)
                    .map(|(name, value)| (name, value.to_string()))
                    .collect()
            });
        }
        RegistryStatus::TaintedWithGruntLauncher => {
            debug!("Installation created by gruntlauncher found. Cleaning");
        }
    }
    clean_registry().await;
    let mut dir_path = OsString::from("/DIR=");
    dir_path.push(&install_path);
    let install_child_status = Command::new(archive_path.as_os_str())
        .arg("/VERYSILENT")
        .arg("/SUPPRESSMSGBOXES")
        .arg(dir_path)
        .arg("/NORESTART")
        .arg("/CURRENTUSER")
        .arg("/NOICONS")
        .arg("/TASKS=\"\"")
        .arg("/LOG")
        .arg("/CLOSEAPPLICATIONS")
        .status();
    let install_child_status = install_child_status.await?;
    debug!("Install Status: {:?}", install_child_status);
    //Clean up newly created registry entry because we dont want it to interfere with future
    //installations
    clean_registry().await;
    //Restore regkey if it existed before the installation by gruntlauncher
    if let Some(saved_reg_key) = &saved_reg_key {
        let (created_reg_key, _) = winreg::HKCU.create_subkey(VSWINREGKEY)?;
        for (name, value) in saved_reg_key {
            created_reg_key.set_value(name, value)?;
        }
    }
    if install_child_status.success() {
        Ok(install_path)
    } else {
        let install_error = VersionsError::InstallerError(format!("{:?}", install_child_status));
        progress
            .send(InstallStatus::Failed(install_error.clone()))
            .await;
        Err(install_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn checksums_match() {
        let result = verify_download(
            "2370eb6cac1b10c7990e4678b4d3c87e".to_string(),
            PathBuf::from(
                "/home/renarin/.cache/gruntlauncher/1.22.2/vs_client_linux-x64_1.22.2.tar.gz",
            ),
        )
        .await
        .unwrap();
        assert!(result)
    }
}
