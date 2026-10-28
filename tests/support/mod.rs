use std::path::PathBuf;

use gruntlauncher::core::{instance::GruntInstance, version::GameVersion};
use uuid::Uuid;

#[cfg(target_os = "macos")]
pub const EXECUTABLE: &str = "Vintagestory";
#[cfg(target_os = "linux")]
pub const EXECUTABLE: &str = "run.sh";
#[cfg(target_os = "windows")]
pub const EXECUTABLE: &str = "Vintagestory.exe";

pub enum TestGame {
    Diagnostics,
    Failing,
}

pub struct Installation {
    pub root: PathBuf,
    pub game: PathBuf,
    pub instance: GruntInstance,
}

impl Installation {
    pub fn new(test_game: Option<TestGame>) -> Self {
        let root = std::env::temp_dir().join(format!("gruntlauncher launch {}", Uuid::new_v4()));
        let game = root.join("game directory");
        #[cfg(target_os = "macos")]
        let game = game.join("Vintage Story.app");
        std::fs::create_dir_all(&game).unwrap();
        if let Some(test_game) = test_game {
            // A native fixture exercises process launching on Windows as well as Unix.
            let source = root.join("game.rs");
            std::fs::write(&source, include_str!("fixtures/game.rs")).unwrap();
            let executable = game.join(EXECUTABLE);
            let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
            let mut command = std::process::Command::new(compiler);
            command
                .arg("--crate-name=gruntlauncher_test_game")
                .arg(&source)
                .arg("-o")
                .arg(&executable);
            if matches!(test_game, TestGame::Failing) {
                command.arg("--cfg=game_failure");
            }
            let output = command.output().expect("compile the native test game");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
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

    pub fn instances(&self) -> PathBuf {
        self.root.join("instance directory")
    }

    pub fn log(&self) -> PathBuf {
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
