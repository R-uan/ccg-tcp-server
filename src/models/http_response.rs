use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PartialPlayerProfile {
    pub id: String,
    pub level: u32,
    pub username: String,
}
