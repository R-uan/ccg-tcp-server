use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ConnRequest {
    pub id: String,
    pub token: String,
    pub current_deck_id: String,
}
