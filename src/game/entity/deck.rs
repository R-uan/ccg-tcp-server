use std::collections::HashMap;
use crate::game::entity::card::{Card, CardRef, CardView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Deck {
    pub id: String,
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub cards: Vec<CardRef>,
}

impl Deck {
    pub fn create_view(&self, cards: &HashMap<String, Card>, owner_id: &str) -> DeckView {
        let mut card_views: HashMap<String, CardView> = HashMap::new();
        for card in &self.cards {
            let full_card = cards.get(&card.id).unwrap();
            let view = CardView::create_view(full_card, owner_id.to_string());
            card_views.insert(card.id.to_string(), view);
        }
        
        DeckView {
            card_views,
            id: self.id.clone(),
            name: self.name.to_string(),
            player_id: self.player_id.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeckView {
    pub id: String,
    pub player_id: String,
    pub name: String,
    pub card_views: HashMap<String, CardView>,
}
