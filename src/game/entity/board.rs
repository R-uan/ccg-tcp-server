use serde::{Deserialize, Serialize};
use crate::game::entity::card::CardRef;

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
