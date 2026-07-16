use bytes::Bytes;
use image::ImageError;
use std::{fmt::Debug, path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::{io, task::JoinError};

use crate::services::HTTP;
#[derive(Clone, Debug, Error)]
pub enum ImagesError {
    #[error("request to the server failed: {0}")]
    Reqwest(Arc<reqwest::Error>),

    #[error("Error while loading image from bytes: {0}")]
    ImageError(Arc<ImageError>),

    #[error("Tokio task join error: {0}")]
    JoinError(Arc<JoinError>),

    #[error("IO Error")]
    IOError(Arc<io::Error>),
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
impl From<io::Error> for ImagesError {
    fn from(value: io::Error) -> Self {
        Self::IOError(Arc::new(value))
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
pub async fn get_image_bytes(url: String) -> Result<Bytes> {
    Ok(HTTP.get(url).send().await?.bytes().await?)
}
async fn image_from_bytes(b: bytes::Bytes) -> Result<DecodedImage> {
    let decoded_image = tokio::task::spawn_blocking(move || -> Result<DecodedImage> {
        let rgba = image::load_from_memory(&b)?.thumbnail(28, 28).into_rgba8();
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
pub async fn load_image(url: String) -> Result<DecodedImage> {
    let bytes = get_image_bytes(url).await?;
    image_from_bytes(bytes).await
}

pub async fn load_image_local(path: PathBuf) -> Result<DecodedImage> {
    let bytes = bytes::Bytes::from(tokio::fs::read(&path).await?);
    image_from_bytes(bytes).await
}
pub async fn save_image(path: PathBuf, url: String) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = get_image_bytes(url).await?;
    tokio::fs::write(&path, &bytes).await?;
    Ok(path)
}
