use std::sync::Arc;

use thiserror::Error;
use tokio::task::JoinError;
use tracing::info;
use velopack::{sources, Error as VelopackError, UpdateCheck, UpdateInfo, UpdateManager};

use crate::services::update::UpdateStatus::InProgress;

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
#[derive(Clone, Debug)]
pub enum UpdateStatus {
    NotStarted,
    InProgress(i16), //1 to 100
    Complete,
}
pub async fn download_and_apply(
    update: Box<UpdateInfo>,
    progress: &mut sipper::Sender<UpdateStatus>,
) -> Result<()> {
    let (sync_tx, sync_rx) = std::sync::mpsc::channel::<i16>();
    let download_handle = tokio::task::spawn_blocking(move || {
        let manager = manager()?;
        manager.download_updates(&update, Some(sync_tx))?;
        manager.apply_updates_and_restart(&update.TargetFullRelease)?;

        Ok(())
    });

    let (async_tx, mut async_rx) = tokio::sync::mpsc::unbounded_channel::<i16>();
    std::thread::spawn(move || {
        while let Ok(percent) = sync_rx.recv() {
            if async_tx.send(percent).is_err() {
                break;
            }
        }
    });

    while let Some(percent) = async_rx.recv().await {
        if percent == 100 {
            progress.send(UpdateStatus::Complete).await;
        } else {
            progress.send(UpdateStatus::InProgress(percent)).await;
        }
    }
    download_handle.await?
}
