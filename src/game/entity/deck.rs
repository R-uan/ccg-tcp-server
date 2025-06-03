use serde::{Deserialize, Serialize};
use crate::game::entity::card::CardRef;

#[derive(Debug, Deserialize, Serialize)]
pub struct Deck {
    pub id: String,
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub cards: Vec<CardRef>,
}
