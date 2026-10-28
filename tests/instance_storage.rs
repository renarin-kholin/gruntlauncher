use gruntlauncher::{
    core::{instance::GruntInstance, version::GameVersion},
    services::instance::{InstancesError, save_instance},
};
use uuid::Uuid;

#[tokio::test]
async fn saving_preserves_config_creation_io_errors() {
    let root = std::env::temp_dir().join(format!("gruntlauncher save {}", Uuid::new_v4()));
    let instance = GruntInstance {
        name: "Save test".into(),
        id: Uuid::new_v4(),
        mods: vec![],
        version: GameVersion::local(semver::Version::new(1, 0, 0), &root.join("game")),
    };
    let config = root.join(instance.id.to_string()).join("instance.toml");
    // A directory in place of the config file makes creation fail on every platform.
    std::fs::create_dir_all(&config).unwrap();
    let expected_error = std::fs::File::create(&config).unwrap_err().kind();

    let error = save_instance(instance, root.clone()).await.unwrap_err();
    std::fs::remove_dir_all(root).unwrap();
    let InstancesError::Io(error) = error else {
        panic!("expected the config creation I/O error")
    };
    assert_eq!(error.kind(), expected_error);
}
