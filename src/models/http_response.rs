use serde::{Deserialize, Serialize};
use crate::models::deck::Card;

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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SelectedCardsResponse {
    #[serde(alias = "cards")]
    pub cards: Vec<Card>,
    #[serde(alias = "invalidCardGuid")]
    pub invalid_card_guid: Vec<String>,
    #[serde(alias = "cardsNotFound")]
    pub cards_not_found: Vec<String>,
}