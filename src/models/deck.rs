use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Deck {
    pub id: String,
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub cards: Vec<Card>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub id: String,
    pub amount: u32,
}
