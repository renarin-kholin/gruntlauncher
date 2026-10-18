use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde_json::{Value, json};
use sipper::StreamExt;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream::wrappers::ReadDirStream;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    core::{account::Account, instance::GruntInstance, version::GameVersionSource},
    services::instance::InstancesError::{ClientSettingsError, EditError},
};

#[derive(Error, Debug, Clone)]
pub enum InstancesError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("error during serializing instance: {0}")]
    TomlSerError(#[from] toml::ser::Error),

    #[error("error during deserializing instance: {0}")]
    TomlDeError(#[from] toml::de::Error),

    #[error("Error during patching clientsettings: {0}")]
    ClientSettingsError(String),

    #[error(transparent)]
    SerdeJson(Arc<serde_json::Error>),

    #[error("Error while editing instance")]
    EditError,

    #[error("The game version must be installed before launching this instance.")]
    VersionNotInstalled,

    #[error("Could not launch {executable}: {reason}")]
    LaunchError { executable: PathBuf, reason: String },

    #[error("Game exited with {status}. See {log_path} for details.")]
    GameExited { status: String, log_path: PathBuf },
}

pub type Result<T> = std::result::Result<T, InstancesError>;

impl From<std::io::Error> for InstancesError {
    fn from(value: std::io::Error) -> Self {
        InstancesError::Io(Arc::new(value))
    }
}
impl From<serde_json::Error> for InstancesError {
    fn from(value: serde_json::Error) -> Self {
        InstancesError::SerdeJson(Arc::new(value))
    }
}

pub async fn delete_instance(instance: Uuid, instances_path: PathBuf) -> Result<()> {
    tokio::fs::create_dir_all(&instances_path).await?;
    let instance_path = instances_path.join(instance.to_string());
    tokio::fs::remove_dir_all(&instance_path).await?;
    Ok(())
}
pub async fn load_instances(instances_path: PathBuf) -> Result<Vec<GruntInstance>> {
    tokio::fs::create_dir_all(&instances_path).await?;
    let dir = tokio::fs::read_dir(instances_path).await?;
    let mut dir = ReadDirStream::new(dir);
    let mut instances = vec![];
    while let Some(entry) = dir.next().await {
        let entry = entry?;
        if let Ok(instance_config) =
            tokio::fs::read_to_string(entry.path().join("instance.toml")).await
        {
            match toml::from_str::<GruntInstance>(&instance_config) {
                Ok(instance) => {
                    info!(instance_name = instance.name, "Loaded instance config");
                    instances.push(instance)
                }
                Err(error) => {
                    error!(%error, "Error trying to parse instance config");
                }
            }
        }
    }
    Ok(instances)
}

pub async fn add_instance(
    instance: GruntInstance,
    instances_path: PathBuf,
) -> Result<GruntInstance> {
    let instance_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(&instance_path).await?;
    let mut instance_config =
        tokio::fs::File::create_new(instance_path.join("instance.toml")).await?;
    instance_config
        .write_all((toml::to_string(&instance)?).as_bytes())
        .await?;

    Ok(instance)
}
pub async fn save_instance(instance: GruntInstance, instances_path: PathBuf) -> Result<()> {
    let instance_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(&instance_path).await?;
    if let Ok(mut instance_config) =
        tokio::fs::File::create(instance_path.join("instance.toml")).await
    {
        instance_config
            .write_all((toml::to_string(&instance)?).as_bytes())
            .await?;
    } else {
        error!("Trying to edit an instance but its config doesn't exist.");
        return Err(EditError);
    }
    Ok(())
}

pub async fn patch_client_settings(instance_path: PathBuf, account: Account) -> Result<()> {
    let clientsettings_file = instance_path.join("clientsettings.json");
    let root_text = match tokio::fs::read_to_string(&clientsettings_file).await {
        Ok(text) => text,
        Err(e) if e.kind() == tokio::io::ErrorKind::NotFound => "{}".to_string(),
        Err(e) => return Err(e.into()),
    };
    let mut root: Value = serde_json::from_str(&root_text)?;
    let ss = root
        .as_object_mut()
        .ok_or_else(|| ClientSettingsError("clientsettings.json root is not an object".into()))?
        .entry("stringsettings")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| ClientSettingsError("stringsettings exists but is not an object".into()))?;
    ss.insert("sessionkey".to_string(), json!(account.sessionkey));
    ss.insert(
        "sessionsignature".to_string(),
        json!(account.sessionsignature),
    );
    ss.insert("playeruid".to_string(), json!(account.uid));
    ss.insert("playername".to_string(), json!(account.username));
    tokio::fs::write(&clientsettings_file, serde_json::to_string_pretty(&root)?).await?;
    Ok(())
}
pub async fn launch_instance(
    instance: GruntInstance,
    instances_path: PathBuf,
    account: Option<Account>,
) -> Result<()> {
    let GameVersionSource::Local(game) = instance.version.source else {
        return Err(InstancesError::VersionNotInstalled);
    };
    let game_directory = tokio::fs::canonicalize(&game.path).await?;
    #[cfg(target_os = "macos")]
    let game_directory = macos_game_directory(&game_directory)?;
    let run_path = game_executable(&game_directory);
    let data_path = instances_path.join(instance.id.to_string());
    tokio::fs::create_dir_all(data_path.join("Mods")).await?;
    let data_path = tokio::fs::canonicalize(data_path).await?;
    let mods_path = data_path.join("Mods");
    if let Some(account) = account {
        patch_client_settings(data_path.clone(), account).await?;
        info!("Successfully patched clientsettings.json");
    }

    let log_path = data_path.join("launch.log");
    let output = tokio::fs::File::create(&log_path).await?.into_std().await;
    let stderr = output.try_clone()?;
    info!(executable = %run_path.display(), instance = %instance.name,
        log = %log_path.display(), "Launching game");
    let status = Command::new(&run_path)
        .current_dir(game_directory)
        .arg("--dataPath")
        .arg(data_path)
        .arg("--addModPath")
        .arg(mods_path)
        .stdout(output)
        .stderr(stderr)
        .status()
        .await
        .map_err(|error| InstancesError::LaunchError {
            executable: run_path,
            reason: error.to_string(),
        })?;
    if !status.success() {
        return Err(InstancesError::GameExited {
            status: status.to_string(),
            log_path,
        });
    }
    Ok(())
}

fn game_executable(game_directory: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    return game_directory.join("Vintagestory");
    #[cfg(target_os = "linux")]
    return game_directory.join("run.sh");
    #[cfg(target_os = "windows")]
    return game_directory.join("Vintagestory.exe");
}

#[cfg(target_os = "macos")]
fn macos_game_directory(installation: &Path) -> Result<PathBuf> {
    let bundle = installation.join("Vintage Story.app");
    if bundle.is_dir() {
        return Ok(bundle);
    }
    if installation
        .extension()
        .is_some_and(|extension| extension == "app")
    {
        return Ok(installation.into());
    }

    Err(InstancesError::LaunchError {
        executable: game_executable(installation),
        reason: "The macOS app bundle is missing. Reinstall this game version to restore Vintage Story.app and its Retina display settings.".into(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::core::version::GameVersion;
    use std::os::unix::fs::PermissionsExt;

    struct Installation {
        root: PathBuf,
        game: PathBuf,
        instance: GruntInstance,
    }

    impl Installation {
        fn new(script: Option<&str>) -> Self {
            let root =
                std::env::temp_dir().join(format!("gruntlauncher launch {}", Uuid::new_v4()));
            let game = root.join("game directory");
            #[cfg(target_os = "macos")]
            let game = game.join("Vintage Story.app");
            std::fs::create_dir_all(&game).unwrap();
            if let Some(script) = script {
                let executable = game_executable(&game);
                std::fs::write(&executable, script).unwrap();
                std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
            Self {
                instance: GruntInstance {
                    name: "Launch test".into(),
                    id: Uuid::new_v4(),
                    mods: vec![],
                    version: GameVersion::local(
                        semver::Version::new(1, 0, 0),
                        &root.join("game directory"),
                    ),
                },
                root,
                game,
            }
        }

        fn instances(&self) -> PathBuf {
            self.root.join("instance directory")
        }

        fn log(&self) -> PathBuf {
            self.instances()
                .join(self.instance.id.to_string())
                .join("launch.log")
        }
    }

    impl Drop for Installation {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn macos_accepts_direct_bundles_and_rejects_stripped_installations() {
        let mut installation = Installation::new(Some("#!/bin/sh\nexit 0\n"));
        installation.instance.version =
            GameVersion::local(semver::Version::new(1, 0, 0), &installation.game);
        launch_instance(
            installation.instance.clone(),
            installation.instances(),
            None,
        )
        .await
        .unwrap();

        let stripped = installation.root.join("stripped");
        std::fs::rename(&installation.game, &stripped).unwrap();
        installation.instance.version =
            GameVersion::local(semver::Version::new(1, 0, 0), &stripped);
        let error = launch_instance(
            installation.instance.clone(),
            installation.instances(),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Reinstall this game version"));
    }

    #[tokio::test]
    async fn launch_uses_game_directory_preserves_spaced_arguments_and_logs_output() {
        let installation = Installation::new(Some(
            "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\"\nprintf 'runtime diagnostic\\n' >&2\n",
        ));
        launch_instance(
            installation.instance.clone(),
            installation.instances(),
            None,
        )
        .await
        .unwrap();
        let output = std::fs::read_to_string(installation.log()).unwrap();
        let data = installation
            .instances()
            .join(installation.instance.id.to_string())
            .canonicalize()
            .unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            vec![
                installation
                    .game
                    .canonicalize()
                    .unwrap()
                    .display()
                    .to_string(),
                "--dataPath".to_string(),
                data.display().to_string(),
                "--addModPath".to_string(),
                data.join("Mods").display().to_string(),
                "runtime diagnostic".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn nonzero_exit_is_reported_with_a_diagnostic_log() {
        let installation =
            Installation::new(Some("#!/bin/sh\nprintf 'runtime failed\\n' >&2\nexit 23\n"));
        let error = launch_instance(
            installation.instance.clone(),
            installation.instances(),
            None,
        )
        .await
        .unwrap_err();
        let InstancesError::GameExited { status, log_path } = error else {
            panic!("expected game exit failure")
        };
        assert!(status.contains("23"));
        assert_eq!(
            std::fs::read_to_string(log_path).unwrap(),
            "runtime failed\n"
        );
    }

    #[tokio::test]
    async fn missing_executable_and_uninstalled_version_are_errors() {
        let mut installation = Installation::new(None);
        let error = launch_instance(
            installation.instance.clone(),
            installation.instances(),
            None,
        )
        .await
        .unwrap_err();
        let InstancesError::LaunchError { executable, .. } = error else {
            panic!("expected spawn failure")
        };
        assert_eq!(
            executable,
            game_executable(&installation.game.canonicalize().unwrap())
        );
        installation.instance.version = GameVersion::remote(
            semver::Version::new(1, 0, 0),
            "game.tar.gz".into(),
            "https://example.invalid/game.tar.gz".into(),
            "test-checksum".into(),
        );
        assert!(matches!(
            launch_instance(
                installation.instance.clone(),
                installation.instances(),
                None
            )
            .await,
            Err(InstancesError::VersionNotInstalled)
        ));
    }
}
