use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::player::Player;
use crate::models::game_action::GameAction;
use crate::models::{deck::Card, views::PrivatePlayerView};
use crate::utils::logger::Logger;

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
        return Box::new(b"Pretend this is the wrapped game state".to_owned());
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
            Logger::warn("[GAME STATE] Both players are already connected");
        }

        self.players.insert(player.id.clone(), player_view_guard);
    }

    /// Fetches the full details of the cards from both player's deck and store it in the game state.
    pub async fn fetch_cards_details(&mut self, cards: Vec<&str>) {}

    /// Store a card in the game state.
    pub async fn add_card(&self, card: Card) {
        let mut card_vec = self.game_cards.write().await;
        card_vec.insert(card.id.to_string(), card);
    }

    pub async fn apply_actions(&self, actions: Vec<GameAction>) {}
}
