use serde::{Deserialize, Serialize};

use super::deck::CardRef;

#[derive(Serialize, Clone)]
pub struct GameStateView {
    pub turn: u32,
    pub red_player: PlayerView,
    pub blue_player: PlayerView,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerView {
    pub id: String,
    pub health: i32,
    pub mana: u32,

    pub hand_size: usize,
    pub deck_size: usize,
    pub current_hand: [Option<CardView>; 10],

    pub board: BoardView,
    pub graveyard_size: usize,
    pub graveyard: GraveyardView,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct CardView {
    pub id: String,
    pub hand_id: u32,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub card_type: String,
    pub effects: Vec<String>,
    pub owner_id: String,
    pub is_exhausted: bool,
    pub position: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct BoardView {
    pub creatures: [Option<CardRef>; 6],
    pub artifacts: [Option<CardRef>; 3],
    pub enchantments: [Option<CardRef>; 3],
}

impl Default for BoardView {
    fn default() -> Self {
        Self {
            artifacts: [None, None, None],
            enchantments: [None, None, None],
            creatures: [None, None, None, None, None, None],
        }
    }
}

#[derive(Serialize, Clone, Deserialize, Debug, Default)]
pub struct GraveyardView {
    pub creatures: Vec<CardRef>,
    pub artifacts: Vec<CardRef>,
    pub enchantments: Vec<CardRef>,
}
