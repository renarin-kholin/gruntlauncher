use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
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
}
impl From<reqwest::Error> for ModsError {
    fn from(value: reqwest::Error) -> Self {
        ModsError::Reqwest(Arc::new(value))
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
