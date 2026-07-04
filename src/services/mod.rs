use std::sync::LazyLock;

pub mod account;
pub mod config;
pub mod game_mod;
pub mod image;
pub mod instance;
pub mod version;

pub static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("gruntlauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Failed to build reqwest client")
});
