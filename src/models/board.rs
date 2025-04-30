use serde::{Deserialize, Serialize};

use super::deck::CardRef;

#[derive(Clone, Serialize, Deserialize)]
pub struct Board {
    pub creatures: [Option<CardRef>; 6],
    pub artifacts: [Option<CardRef>; 3],
    pub enchantments: [Option<CardRef>; 3],
}

impl Default for Board {
    fn default() -> Self {
        Self {
            creatures: [None, None, None, None, None, None],
            artifacts: [None, None, None],
            enchantments: [None, None, None],
        }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Graveyard {
    pub creatures: Vec<CardRef>,
    pub artifacts: Vec<CardRef>,
    pub enchantments: Vec<CardRef>,
}
