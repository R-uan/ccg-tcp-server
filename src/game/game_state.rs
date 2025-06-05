use crate::game::entity::card::{Card, CardRef};
use crate::game::entity::player::{Player, PlayerView, PublicPlayerView};
use crate::logger;
use crate::models::game_action::GameAction;
use crate::utils::errors::{CardRequestError, GameLogicError};
use crate::utils::logger::Logger;
use std::{collections::HashMap, sync::Arc};
use serde::Serialize;
use tokio::sync::RwLock;
use crate::game::lua_context::LuaContext;
use crate::models::client_requests::PlayCardRequest;
use crate::tcp::client::Client;
use crate::tcp::server::ServerInstance;

pub struct GameState {
    pub rounds: u32,
    pub red_first: bool,
    pub red_player: String,
    pub blue_player: String,
    pub ongoing: Arc<RwLock<bool>>,
    pub player_views: Arc<RwLock<HashMap<String, Arc<RwLock<PlayerView>>>>>
}

impl GameState {
    pub fn new_game() -> Self {
        Self {
            rounds: 0,
            red_first: true,
            red_player: String::new(),
            blue_player: String::new(),
            ongoing: Arc::new(RwLock::new(true)),
            player_views: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    /// Wraps the game state into a byte array for transmission or storage.
    pub fn wrap_game_state(&self) -> Box<[u8]> {
        Box::new(b"Pretend this is the wrapped game state".to_owned())
    }

    pub async fn apply_actions(&self, actions: Vec<GameAction>) {}
}

#[derive(Serialize, Clone)]
pub struct PrivateGameStateView {
    pub turn: u32,
    pub red_player: PlayerView,
    pub blue_player: PlayerView,
}

#[derive(Serialize, Clone)]
pub struct PublicGameStateView {
    pub turn: u32,
    pub red_player: PublicPlayerView,
    pub blue_player: PublicPlayerView,
}