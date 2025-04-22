use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    models::{board::Board, board::Cemetery},
    tcp::client::Client,
};

use super::player::Player;

pub struct GameState {
    pub rounds: u32,
    pub red_first: bool,

    pub red_board: Board,
    pub red_player: Option<Arc<RwLock<Player>>>,
    pub red_cemetery: Cemetery,

    pub blue_board: Board,
    pub blue_player: Option<Arc<RwLock<Player>>>,
    pub blue_cemetery: Cemetery,
}

impl GameState {
    pub fn new_game() -> Self {
        return GameState {
            rounds: 0,
            red_first: true,

            blue_player: None,
            blue_board: Board::default(),
            blue_cemetery: Cemetery::default(),

            red_player: None,
            red_board: Board::default(),
            red_cemetery: Cemetery::default(),
        };
    }

    pub fn wrap_game_state(&self) -> Box<[u8]> {
        let xd = b"placeholder: to do gamestate";
        return xd.to_vec().into_boxed_slice();
    }

    pub async fn add_blue_player(&mut self, client: &Client) {
        let mut player_lock = client.player.write().await;
        let player = player_lock.take().unwrap();
        self.blue_player = Some(Arc::new(RwLock::new(player)));
    }

    pub async fn add_red_player(&mut self, client: &Client) {
        let mut player_lock = client.player.write().await;
        let player = player_lock.take().unwrap();
        self.red_player = Some(Arc::new(RwLock::new(player)));
    }
}
