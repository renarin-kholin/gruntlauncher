use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::process::Command;
use tracing::{error, info};
use uuid::Uuid;

use crate::core::{account::Account, instance::GruntInstance, version::GameVersionSource};

use super::{InstancesError, Result};

pub async fn launch_instance(
    instance: GruntInstance,
    instances_path: PathBuf,
    account: Option<Account>,
) -> Result<()> {
    let GameVersionSource::Local(game) = instance.version.source else {
        return Err(InstancesError::VersionNotInstalled);
    };
    let target = resolve_launch_target(&game.path).await?;
    let data_path = prepare_instance(&instances_path, instance.id, account).await?;
    let log_path = data_path.join("launch.log");

    let mut command = Command::new(&target.executable);
    command
        .current_dir(&target.working_directory)
        .arg("--dataPath")
        .arg(&data_path)
        .arg("--addModPath")
        .arg(data_path.join("Mods"));
    redirect_game_output(&mut command, &log_path).await?;

    info!(executable = %target.executable.display(), instance = %instance.name,
        log = %log_path.display(), "Launching game");
    let status = command
        .status()
        .await
        .map_err(|error| InstancesError::LaunchError {
            executable: target.executable,
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

async fn prepare_instance(
    instances_path: &Path,
    instance_id: Uuid,
    account: Option<Account>,
) -> Result<PathBuf> {
    let data_path = instances_path.join(instance_id.to_string());
    tokio::fs::create_dir_all(data_path.join("Mods")).await?;
    let data_path = tokio::fs::canonicalize(data_path).await?;
    if let Some(account) = account {
        match patch_account_settings(data_path.clone(), account).await {
            Ok(()) => info!("Successfully patched clientsettings.json"),
            Err(error) => {
                error!(%error, "Failed to update account settings; launching with existing settings")
            }
        }
    }
    Ok(data_path)
}

async fn redirect_game_output(command: &mut Command, log_path: &Path) -> Result<()> {
    let output = tokio::fs::File::create(log_path).await?.into_std().await;
    let stderr = output.try_clone()?;
    command.stdout(output).stderr(stderr);
    Ok(())
}

pub async fn patch_account_settings(instance_path: PathBuf, account: Account) -> Result<()> {
    let settings_file = instance_path.join("clientsettings.json");
    let settings_text = match tokio::fs::read_to_string(&settings_file).await {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
        Err(error) => return Err(error.into()),
    };
    let mut settings_root: Value = serde_json::from_str(&settings_text)?;
    let settings = settings_root.as_object_mut().ok_or_else(|| {
        InstancesError::ClientSettingsError("clientsettings.json root is not an object".into())
    })?;
    let string_settings = settings
        .entry("stringsettings")
        .or_insert_with(|| json!({}));
    let string_settings = string_settings.as_object_mut().ok_or_else(|| {
        InstancesError::ClientSettingsError("stringsettings exists but is not an object".into())
    })?;
    string_settings.insert("sessionkey".to_string(), json!(account.sessionkey));
    string_settings.insert(
        "sessionsignature".to_string(),
        json!(account.sessionsignature),
    );
    string_settings.insert("playeruid".to_string(), json!(account.uid));
    string_settings.insert("playername".to_string(), json!(account.username));
    tokio::fs::write(
        &settings_file,
        serde_json::to_string_pretty(&settings_root)?,
    )
    .await?;
    Ok(())
}

struct LaunchTarget {
    executable: PathBuf,
    working_directory: PathBuf,
}

async fn resolve_launch_target(installation: &Path) -> Result<LaunchTarget> {
    let installation = tokio::fs::canonicalize(installation).await?;
    #[cfg(target_os = "linux")]
    return Ok(linux_launch_target(installation));
    #[cfg(target_os = "macos")]
    return macos_launch_target(installation);
    #[cfg(target_os = "windows")]
    return Ok(windows_launch_target(installation));
}

#[cfg(target_os = "linux")]
fn linux_launch_target(installation: PathBuf) -> LaunchTarget {
    LaunchTarget {
        executable: installation.join("run.sh"),
        working_directory: installation,
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_target(installation: PathBuf) -> Result<LaunchTarget> {
    let bundle = installation.join("Vintage Story.app");
    let working_directory = if bundle.is_dir() {
        bundle
    } else if installation
        .extension()
        .is_some_and(|extension| extension == "app")
    {
        installation
    } else {
        return Err(InstancesError::LaunchError {
            executable: installation.join("Vintagestory"),
            reason: "The macOS app bundle is missing. Reinstall this game version to restore Vintage Story.app and its Retina display settings.".into(),
        });
    };
    Ok(LaunchTarget {
        executable: working_directory.join("Vintagestory"),
        working_directory,
    })
}

#[cfg(target_os = "windows")]
fn windows_launch_target(installation: PathBuf) -> LaunchTarget {
    LaunchTarget {
        executable: installation.join("Vintagestory.exe"),
        working_directory: installation,
    }
}
