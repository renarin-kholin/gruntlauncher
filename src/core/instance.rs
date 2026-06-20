use uuid::Uuid;

use crate::core::game_mod::Mod;

pub type InstanceId = Uuid;
#[derive(Clone, Debug)]
pub struct GruntInstance {
    pub name: String,
    pub id: InstanceId,
    pub mods: Vec<Mod>,
}
