use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{
    board::{Board, Graveyard},
    deck::Card,
};

use super::{lua_context::PlayerView, player::Player, script_manager::ScriptManager};

pub struct GameState {
    pub rounds: u32,
    pub red_first: bool,
    pub curr_turn: String, // Blue or Red
    pub lua_scripts: Arc<RwLock<ScriptManager>>,
    pub red_player: Option<Arc<RwLock<PlayerView>>>,
    pub blue_player: Option<Arc<RwLock<PlayerView>>>,
    pub game_cards: Arc<RwLock<Vec<Card>>>,
}

impl GameState {
    pub fn new_game(scripts: Arc<RwLock<ScriptManager>>) -> Self {
        return Self {
            rounds: 0,
            red_first: true,
            red_player: None,
            blue_player: None,
            lua_scripts: scripts,
            curr_turn: String::from("Red"),
            game_cards: Arc::new(RwLock::new(Vec::new())),
        };
    }

    pub fn add_players(&mut self, blue: Arc<&Player>, red: Arc<&Player>) {
        let blue_player = PlayerView {
            id: blue.id.clone(),
            health: 30,
            mana: 1,

            hand_size: 0,
            board: Board::default(),
            deck_size: blue.current_deck.cards.len(),
            graveyard: Graveyard::default(),
            graveyard_size: 0,
        };

        let red_player = PlayerView {
            id: red.id.clone(),
            health: 30,
            mana: 1,

            hand_size: 0,
            board: Board::default(),
            deck_size: red.current_deck.cards.len(),
            graveyard: Graveyard::default(),
            graveyard_size: 0,
        };

        self.blue_player = Some(Arc::new(RwLock::new(blue_player)));
        self.red_player = Some(Arc::new(RwLock::new(red_player)));
    }

    pub async fn fetch_cards_details(&mut self, cards: Vec<&str>) {}
}
