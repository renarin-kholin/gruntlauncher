use std::sync::Arc;

use thiserror::Error;
use tokio::task::JoinError;
use tracing::info;
use velopack::{sources, Error as VelopackError, UpdateCheck, UpdateInfo, UpdateManager};

#[derive(Clone, Debug, Error)]
pub enum UpdatesError {
    #[error("updater error: {0}")]
    Velopack(Arc<VelopackError>),

    #[error("Tokio task join error: {0}")]
    JoinError(Arc<JoinError>),
}

impl From<VelopackError> for UpdatesError {
    fn from(value: VelopackError) -> Self {
        Self::Velopack(Arc::new(value))
    }
}
impl From<JoinError> for UpdatesError {
    fn from(value: JoinError) -> Self {
        Self::JoinError(Arc::new(value))
    }
}
pub type Result<T> = std::result::Result<T, UpdatesError>;

fn manager() -> std::result::Result<UpdateManager, VelopackError> {
    let source = sources::GithubSource::new(env!("CARGO_PKG_REPOSITORY"), None, false);
    UpdateManager::new(source, None, None)
}

pub async fn check_for_update() -> Result<Option<Box<UpdateInfo>>> {
    tokio::task::spawn_blocking(|| {
        let manager = match manager() {
            Ok(manager) => manager,
            Err(VelopackError::NotInstalled(reason)) => {
                info!("Not a Velopack install, skipping update check: {reason}");
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        match manager.check_for_updates()? {
            UpdateCheck::UpdateAvailable(update) => Ok(Some(update)),
            UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => Ok(None),
        }
    })
    .await?
}

pub async fn download_and_apply(update: Box<UpdateInfo>) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let manager = manager()?;
        manager.download_updates(&update, None)?;
        manager.apply_updates_and_restart(&update.TargetFullRelease)?;
        Ok(())
    })
    .await?
}
