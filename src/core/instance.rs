use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{game_mod::GameMod, version::GameVersion};

pub type InstanceId = Uuid;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GruntInstance {
    pub name: String,
    pub id: InstanceId,
    pub mods: Vec<GameMod>,
    pub version: GameVersion,
}
