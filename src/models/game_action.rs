use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameAction {
    DealDamage { target: String, amount: u32 },
    Heal { target: String, amount: u32 },
    Summon { id: String, position: String }
}