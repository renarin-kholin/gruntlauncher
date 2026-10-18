use std::path::Path;

use gruntlauncher::{
    core::{
        account::{Account, AccountStatus},
        version::GameVersion,
    },
    services::instance::{InstancesError, launch_instance, patch_client_settings},
};

mod support;

use support::{EXECUTABLE, Installation, TestGame};

#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_accepts_direct_bundles_and_rejects_stripped_installations() {
    let mut installation = Installation::new(Some(TestGame::Diagnostics));
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
    installation.instance.version = GameVersion::local(semver::Version::new(1, 0, 0), &stripped);
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
    let installation = Installation::new(Some(TestGame::Diagnostics));
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
    let mut lines = output.lines();
    // Windows may report the working directory without canonicalize's verbatim prefix.
    let working_directory = Path::new(lines.next().unwrap()).canonicalize().unwrap();
    assert_eq!(working_directory, installation.game.canonicalize().unwrap());
    assert_eq!(
        lines.collect::<Vec<_>>(),
        vec![
            "--dataPath".to_string(),
            data.display().to_string(),
            "--addModPath".to_string(),
            data.join("Mods").display().to_string(),
            "runtime diagnostic".to_string(),
        ]
    );
}

#[tokio::test]
async fn readonly_account_settings_do_not_prevent_launching() {
    let installation = Installation::new(Some(TestGame::Diagnostics));
    let data_path = installation
        .instances()
        .join(installation.instance.id.to_string());
    std::fs::create_dir_all(&data_path).unwrap();
    let settings_file = data_path.join("clientsettings.json");
    let settings =
        r#"{"stringsettings":{"playername":"Existing player"},"intSettings":{"gameWindowMode":1}}"#;
    std::fs::write(&settings_file, settings).unwrap();
    let original_permissions = std::fs::metadata(&settings_file).unwrap().permissions();
    let mut readonly_permissions = original_permissions.clone();
    readonly_permissions.set_readonly(true);
    std::fs::set_permissions(&settings_file, readonly_permissions).unwrap();
    let account = Account::new(
        "Selected player",
        "player@example.invalid",
        AccountStatus::Ok,
        "test session".into(),
        "test signature".into(),
        "test uid".into(),
    );

    // Confirm that patching really fails, then exercise the launch fallback.
    let patch_result = patch_client_settings(data_path, account.clone()).await;
    let launch_result = launch_instance(
        installation.instance.clone(),
        installation.instances(),
        Some(account),
    )
    .await;
    // Restore permissions before assertions so Windows can clean up even on failure.
    std::fs::set_permissions(&settings_file, original_permissions).unwrap();

    let InstancesError::Io(error) = patch_result.unwrap_err() else {
        panic!("expected a settings write I/O error")
    };
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    launch_result.unwrap();
    assert_eq!(std::fs::read_to_string(settings_file).unwrap(), settings);
    assert!(
        std::fs::read_to_string(installation.log())
            .unwrap()
            .contains("runtime diagnostic")
    );
}

#[tokio::test]
async fn nonzero_exit_is_reported_with_a_diagnostic_log() {
    let installation = Installation::new(Some(TestGame::Failing));
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
        installation.game.canonicalize().unwrap().join(EXECUTABLE)
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
