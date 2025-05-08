use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::models::{
    deck::Card,
    views::{BoardView, GraveyardView, PlayerView},
};

use super::{player::Player, script_manager::ScriptManager};

pub struct GameState {
    pub rounds: u32,
    pub red_first: bool,
    pub curr_turn: String, // Blue or Red
    pub red_player: String,
    pub blue_player: String,
    pub game_cards: Arc<RwLock<Vec<Card>>>,
    pub lua_scripts: Arc<RwLock<ScriptManager>>,
    pub players: HashMap<String, Arc<RwLock<PlayerView>>>,
}

impl GameState {
    pub fn new_game(scripts: Arc<RwLock<ScriptManager>>) -> Self {
        Self {
            rounds: 0,
            red_first: true,
            lua_scripts: scripts,
            players: HashMap::new(),
            red_player: String::new(),
            blue_player: String::new(),
            curr_turn: String::from("Red"),
            game_cards: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_players(&mut self, blue: Arc<&Player>, red: Arc<&Player>) {
        let blue_player = PlayerView::from_player(blue);
        let red_player = PlayerView::from_player(red);

        self.blue_player = blue_player.id.to_owned();
        self.red_player = red_player.id.to_owned();

        self.players.insert(
            blue_player.id.to_owned(),
            Arc::new(RwLock::new(blue_player)),
        );
        self.players
            .insert(red_player.id.to_owned(), Arc::new(RwLock::new(red_player)));
    }

    pub async fn fetch_cards_details(&mut self, cards: Vec<&str>) {}
}
