use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Deck {
    pub id: String,
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub cards: Vec<CardRef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CardRef {
    pub id: String,
    pub amount: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub description: String,
    pub play_cost: u32,
    pub attack: u32,
    pub health: u32,
    pub rarity: u16,

    // These will contain lua function names I guess
    pub on_play: Vec<String>,
    pub on_draw: Vec<String>,

    pub on_attack: Vec<String>,
    pub on_hit: Vec<String>,

    pub on_turn_start: Vec<String>,
    pub on_turn_end: Vec<String>,

    pub on_death: Vec<String>,
    pub on_ally_death: Vec<String>,
    pub on_enemy_death: Vec<String>,
}
