use crate::game::entity::card::{Card, CardRef};
use crate::game::entity::player::{Player, PrivatePlayerView, PublicPlayerView};
use crate::logger;
use crate::models::game_action::GameAction;
use crate::utils::errors::CardRequestError;
use crate::utils::logger::Logger;
use std::{collections::HashMap, sync::Arc};
use serde::Serialize;
use tokio::sync::RwLock;

pub struct GameState {
    pub rounds: u32,
    pub red_first: bool,
    pub curr_turn: String, // Blue or Red
    pub red_player: String,
    pub blue_player: String,
    pub ongoing: Arc<RwLock<bool>>,
    pub game_cards: Arc<RwLock<HashMap<String, Card>>>,
    pub players: HashMap<String, Arc<RwLock<PrivatePlayerView>>>,
}

impl GameState {
    pub fn new_game() -> Self {
        Self {
            rounds: 0,
            red_first: true,
            players: HashMap::new(),
            red_player: String::new(),
            blue_player: String::new(),
            curr_turn: String::from("Red"),
            ongoing: Arc::new(RwLock::new(true)),
            game_cards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Wraps the game state into a byte array for transmission or storage.
    pub fn wrap_game_state(&self) -> Box<[u8]> {
        Box::new(b"Pretend this is the wrapped game state".to_owned())
    }

    /// Adds a player to the game state's hashmap.
    pub async fn add_player(&mut self, player: Arc<Player>) {
        let player_view = PrivatePlayerView::from_player(player.clone());
        let player_view_guard = Arc::new(RwLock::new(player_view));

        if self.blue_player.is_empty() {
            self.blue_player = player.id.clone();
        } else if self.red_player.is_empty() {
            self.red_player = player.id.clone();
        } else {
            logger!(WARN, "[GAME STATE] Both players are already connected");
        }

        self.players.insert(player.id.clone(), player_view_guard);
    }

    /// Fetches the full details of the cards from both player decks and store them in the game state.
    pub async fn fetch_cards_details(&self, cards: Vec<CardRef>) -> Result<(), CardRequestError> {
        let full_cards = Card::request_cards(&cards).await?;
        let mut game_cards_lock = self.game_cards.write().await;

        for card in full_cards {
            let id_clone = card.id.clone();
            game_cards_lock.insert(id_clone, card);
        }

        Ok(())
    }

    /// Store a card in the game state.
    pub async fn add_card(&self, card: Card) {
        let mut card_vec = self.game_cards.write().await;
        card_vec.insert(card.id.to_string(), card);
    }

    pub async fn apply_actions(&self, actions: Vec<GameAction>) {}
}

#[derive(Serialize, Clone)]
pub struct PrivateGameStateView {
    pub turn: u32,
    pub red_player: PrivatePlayerView,
    pub blue_player: PrivatePlayerView,
}

#[derive(Serialize, Clone)]
pub struct PublicGameStateView {
    pub turn: u32,
    pub red_player: PublicPlayerView,
    pub blue_player: PublicPlayerView,
}