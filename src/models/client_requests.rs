use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ConnRequest {
    pub player_id: String,
    pub auth_token: String,
    pub current_deck_id: String,
}
