use thiserror::Error;
#[derive(Debug, Error)]
pub enum GruntError {
    #[error("Error in the Iced runtime: {0:?}")]
    IcedError(#[from] iced::Error),
}

pub type GruntResult<T> = Result<T, GruntError>;
