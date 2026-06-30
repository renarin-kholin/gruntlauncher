use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Debug, Clone, thiserror::Error)]
#[error("Could not determine project directories.")]
pub struct ProjectDirError;

pub fn dirs() -> Result<ProjectDirs, ProjectDirError> {
    ProjectDirs::from("com", "renarin", "gruntlauncher").ok_or(ProjectDirError)
}

pub fn cache_dir() -> Result<PathBuf, ProjectDirError> {
    Ok(dirs()?.cache_dir().to_path_buf())
}

pub fn config_dir() -> Result<PathBuf, ProjectDirError> {
    Ok(dirs()?.config_dir().to_path_buf())
}

pub fn data_dir() -> Result<PathBuf, ProjectDirError> {
    Ok(dirs()?.data_dir().to_path_buf())
}
