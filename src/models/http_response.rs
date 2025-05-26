use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PartialPlayerProfile {
    pub id: String,
    pub level: u32,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AuthenticatedPlayer {
    #[serde(alias = "playerId")]
    pub player_id: String,
    pub username: String,
    #[serde(alias = "isBanned")]
    pub is_banned: bool
}