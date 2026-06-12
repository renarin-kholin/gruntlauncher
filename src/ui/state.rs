use iced_blitzview::Content;

use crate::core::instance::GruntInstance;

pub struct GruntState {
    pub instances: Vec<GruntInstance>,
    pub webview_content: Content,
}
//Temporary initializer with mock data
impl Default for GruntState {
    fn default() -> Self {
        Self {
            instances: vec![
                GruntInstance {
                    name: "Test".to_string()
                };
                20
            ],
            webview_content: Content::new(),
        }
    }
}
