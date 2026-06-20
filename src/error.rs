use thiserror::Error;

use crate::services::instance::LoadInstancesError;
#[derive(Debug, Error)]
pub enum GruntError {
    #[error("Error in the Iced runtime: {0:?}")]
    IcedError(#[from] iced::Error),

    #[error("Error while loading instances: {0:?}")]
    LoadInstancesError(#[from] LoadInstancesError),
}

pub type GruntResult<T> = Result<T, GruntError>;
