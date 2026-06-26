use iced_blitzview::Content;

use crate::core::{config::Config, instance::GruntInstance, version::VersionCatalog};

pub struct GruntState {
    pub instances: Vec<GruntInstance>,
    pub webview_content: Content,
    pub config: Option<Config>,
    pub vs_versions: VersionCatalog,
}
impl Default for GruntState {
    fn default() -> Self {
        Self {
            instances: vec![],
            webview_content: Content::new(),
            config: None,
            vs_versions: VersionCatalog::NotLoaded,
        }
    }
}
