use std::num::NonZeroUsize;

use iced_blitzview::Content;
use lru::LruCache;

use crate::core::{
    account::Account, config::Config, instance::GruntInstance, version::VersionCatalog,
};

pub struct GruntState {
    pub instances: Vec<GruntInstance>,
    pub webview_content: Content,
    pub config: Config,
    pub vs_versions: VersionCatalog,
    pub image_cache: LruCache<i64, iced::widget::image::Handle>,
    pub accounts: Vec<Account>,
    pub selected_account: Option<String>,
}
impl Default for GruntState {
    fn default() -> Self {
        Self {
            instances: vec![],
            webview_content: Content::new(),
            config: Config::default(),
            vs_versions: VersionCatalog::NotLoaded,
            image_cache: LruCache::new(
                NonZeroUsize::new(500).expect("Could not create an LRU Cache for image caching"),
            ),
            accounts: vec![],
            selected_account: None,
        }
    }
}
