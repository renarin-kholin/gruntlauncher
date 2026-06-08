use crate::core::instance::GruntInstance;

pub struct GruntState {
    pub instances: Vec<GruntInstance>,
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
        }
    }
}
