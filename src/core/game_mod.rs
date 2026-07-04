use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ModSource {
    ModDb { modid: i64, release_id: u64 },
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameMod {
    pub source: ModSource,
}
