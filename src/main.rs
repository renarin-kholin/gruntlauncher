#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tracing_subscriber::EnvFilter;

pub mod assets;
pub mod core;
pub mod error;
pub mod paths;
pub mod services;
pub mod ui;

pub use error::GruntError;
pub use error::GruntResult;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        //Check if its on debug profile
        if cfg!(debug_assertions) {
            EnvFilter::new("warn,gruntlauncher=debug")
        } else {
            EnvFilter::new("info,iced_winit=warn")
        }
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
fn main() -> GruntResult<()> {
    // Must run before anything else: handles Velopack install/update/uninstall
    // hooks and may exit or restart the process.
    velopack::VelopackApp::build().run();
    init_tracing();
    ui::app::run()?;
    Ok(())
}
