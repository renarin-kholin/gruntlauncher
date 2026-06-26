use serde::{Deserialize, Serialize};

pub trait GameMod {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ModSource {
    ModDb { mod_id: u64, release_id: u64 },
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mod {
    pub source: ModSource,
}
