pub trait GameMod {}

#[derive(Clone, Debug)]
pub enum ModSource {
    ModDb { mod_id: u64, release_id: u64 },
    Local,
}

#[derive(Clone, Debug)]
pub struct Mod {
    pub source: ModSource,
}
