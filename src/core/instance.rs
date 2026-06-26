use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{game_mod::Mod, version::GameVersion};

pub type InstanceId = Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GruntInstance {
    pub name: String,
    pub id: InstanceId,
    pub mods: Vec<Mod>,
    pub version: GameVersion,
}
