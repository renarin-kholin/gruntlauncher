use std::{path::PathBuf, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use sipper::StreamExt;
use thiserror::Error;
use tracing::debug;

use crate::{assets::VSMODDB, core::version::GameVersion, services::HTTP};
#[derive(Debug, Serialize, Deserialize)]
pub struct ModList {
    statuscode: String,
    mods: Vec<ModListEntry>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModListEntry {
    pub modid: i64,
    pub assetid: i64,
    pub downloads: i64,
    // pub follows: i64,
    // pub trendingpoints: i64,
    // pub comments: i64,
    pub name: String,
    pub summary: String,
    // pub modidstrs: Vec<String>,
    pub author: String,
    pub urlalias: Option<String>,
    pub side: Side,
    #[serde(rename = "type")]
    pub mod_type: Type,
    pub logo: Option<String>,
    pub tags: Vec<String>,
    pub lastreleased: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModDetailResponse {
    #[serde(rename = "mod")]
    pub moddetails: ModDetail,
    pub statuscode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModDetail {
    pub modid: i64,
    pub assetid: i64,
    pub name: String,
    pub text: String,
    pub author: String,
    pub urlalias: Option<String>,
    pub logofilename: Option<String>,
    pub logofile: Option<String>,
    // pub logofiledb: String,
    pub homepageurl: String,
    // pub sourcecodeurl: String,
    // pub trailervideourl: Option<String>,
    // pub issuetrackerurl: Option<String>,
    // pub wikiurl: Option<String>,
    // pub downloads: i64,
    // pub follows: i64,
    // pub trendingpoints: i64,
    // pub comments: i64,
    pub side: Side,
    #[serde(rename = "type")]
    pub mod_type: Type,
    pub created: String,
    pub lastreleased: String,
    pub lastmodified: String,
    pub tags: Vec<String>,
    pub releases: Vec<Release>,
    pub screenshots: Vec<Screenshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Release {
    pub releaseid: i64,
    pub mainfile: String,
    pub filename: String,
    pub fileid: i64,
    pub downloads: i64,
    pub tags: Vec<semver::Version>,
    pub modidstr: String,
    pub modversion: semver::Version,
    pub created: String,
    // pub changelog: String,
}
impl std::fmt::Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.modversion)
    }
}
impl Release {
    pub fn is_compatible(&self, gameversion: &GameVersion) -> Option<Release> {
        if self.tags.contains(&gameversion.version) {
            Some(self.clone())
        } else {
            None
        }
    }
}
pub fn get_compatible_release(mods: &[Release], gameversion: &GameVersion) -> Release {
    let mut compatible_releases = vec![];
    for gamemod in mods {
        if let Some(release) = gamemod.is_compatible(gameversion) {
            compatible_releases.push(release);
        }
    }
    compatible_releases.sort_by(|a, b| b.modversion.cmp(&a.modversion));
    if let Some(release) = compatible_releases.first() {
        release.clone()
    } else {
        mods[0].clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Screenshot {
    pub fileid: i64,
    pub mainfile: String,
    pub filename: String,
    pub thumbnailfilename: String,
    // pub created: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Externaltool,
    Mod,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Both,
    Client,
    Server,
}

pub enum ModSearchState {
    NotStarted,
    Loading,
    Loaded(Vec<ModListEntry>),
    Failed(ModsError),
}

#[derive(Clone, Debug, Error)]
pub enum ModsError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("Failed to build request URL")]
    UrlError,

    #[error("Failed to parse URL")]
    UrlParseError(#[from] url::ParseError),

    #[error("request to the server failed: {0}")]
    Reqwest(Arc<reqwest::Error>),

    #[error("Failed fetching mods from the api")]
    ModAPIError,

    #[error("received response had no content length specified")]
    NoContentLength,
}
impl From<reqwest::Error> for ModsError {
    fn from(value: reqwest::Error) -> Self {
        ModsError::Reqwest(Arc::new(value))
    }
}

impl From<std::io::Error> for ModsError {
    fn from(value: std::io::Error) -> Self {
        ModsError::Io(Arc::new(value))
    }
}
pub type Result<T> = std::result::Result<T, ModsError>;
pub async fn search_mods(query: String) -> Result<Vec<ModListEntry>> {
    let url = reqwest::Url::from_str(VSMODDB)?.join("mods")?;
    debug!("{:?}", url);
    let response = HTTP
        .get(url)
        .query(&[("orderby", "downloads"), ("text", &query)])
        .send()
        .await?;
    let parsed_response: ModList = response.json().await?;

    Ok(parsed_response.mods)
}

#[derive(Clone)]
pub enum ModDetailState {
    NotStarted,
    Loading,
    Loaded(Box<ModDetail>),
    Failed(ModsError),
}
pub async fn get_mod_details(mod_id: String) -> Result<Box<ModDetail>> {
    let url = reqwest::Url::from_str(VSMODDB)?
        .join("mod/")?
        .join(&mod_id)?;
    let response = HTTP.get(url).send().await?;
    let parsed_response: ModDetailResponse = response.json().await?;
    Ok(Box::new(parsed_response.moddetails))
}
#[derive(Debug, Clone)]
pub enum ModDownloadProgress {
    Queued,
    Downloading { total: u64, downloaded: u64 },
    Downloaded,
    Failed(ModsError),
}
pub async fn download_mod(
    dest: PathBuf,
    release: Release,
    progress: &mut sipper::Sender<ModDownloadProgress>,
) -> Result<PathBuf> {
    let result: Result<PathBuf> = {
        tokio::fs::create_dir_all(&dest).await?;
        let temp_file_path = dest.join(release.filename);
        let mut temp_file = tokio::fs::File::create(&temp_file_path).await?;
        let response = HTTP.get(release.mainfile).send().await?;
        let total = response
            .content_length()
            .ok_or(ModsError::NoContentLength)?;
        progress
            .send(ModDownloadProgress::Downloading {
                total,
                downloaded: 0,
            })
            .await;

        let mut byte_stream = response.bytes_stream();
        let mut downloaded = 0;
        while let Some(next_bytes) = byte_stream.next().await {
            let bytes = next_bytes?;
            tokio::io::AsyncWriteExt::write_all(&mut temp_file, &bytes).await?;
            downloaded += bytes.len();
            progress
                .send(ModDownloadProgress::Downloading {
                    downloaded: downloaded as u64,
                    total,
                })
                .await;
        }
        progress.send(ModDownloadProgress::Downloaded).await;
        Ok(temp_file_path)
    };
    match &result {
        Ok(_) => {}
        Err(e) => {
            progress.send(ModDownloadProgress::Failed(e.clone())).await;
        }
    }
    result
}
