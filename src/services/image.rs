use image::ImageError;
use std::{fmt::Debug, sync::Arc};
use thiserror::Error;
use tokio::task::JoinError;

use crate::services::HTTP;
#[derive(Clone, Debug, Error)]
pub enum ImagesError {
    #[error("request to the server failed: {0}")]
    Reqwest(Arc<reqwest::Error>),

    #[error("Error while loading image from bytes: {0}")]
    ImageError(Arc<ImageError>),

    #[error("Tokio task join error: {0}")]
    JoinError(Arc<JoinError>),
}

impl From<reqwest::Error> for ImagesError {
    fn from(value: reqwest::Error) -> Self {
        ImagesError::Reqwest(Arc::new(value))
    }
}
impl From<ImageError> for ImagesError {
    fn from(value: ImageError) -> Self {
        Self::ImageError(Arc::new(value))
    }
}
impl From<JoinError> for ImagesError {
    fn from(value: JoinError) -> Self {
        Self::JoinError(Arc::new(value))
    }
}
pub type Result<T> = std::result::Result<T, ImagesError>;
#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Debug for DecodedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba_len", &self.rgba.len())
            .finish()
    }
}
pub async fn load_image(url: String) -> Result<DecodedImage> {
    let bytes = HTTP.get(url).send().await?.bytes().await?;
    let decoded_image = tokio::task::spawn_blocking(move || -> Result<DecodedImage> {
        let rgba = image::load_from_memory(&bytes)?
            .thumbnail(28, 28)
            .into_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(DecodedImage {
            width,
            height,
            rgba: rgba.into_raw(),
        })
    })
    .await??;
    Ok(decoded_image)
}
