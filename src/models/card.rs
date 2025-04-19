use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub card_id: Uuid,
    pub amount: u8,
}
