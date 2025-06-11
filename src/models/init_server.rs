use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InitServerRequest {
    pub match_id: String,
    pub match_type: String,
    pub players: Vec<PreloadPlayer>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreloadPlayer {
    pub id: String,
    pub deck_id: String,
}