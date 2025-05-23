use std::sync::{Arc};
use mlua::LuaSerdeExt;
use serde::Serialize;
use tokio::sync::RwLock;
use crate::{
    models::views::{CardView, PrivateGameStateView},
};

use super::game_state::GameState;

#[derive(Serialize, Clone)]
pub struct LuaContext {
    pub event: String,
    pub player_turn: String,
    pub action_name: String,

    pub actor_id: String,
    pub actor_view: CardView,
    pub game_state: PrivateGameStateView,
    pub target_id: Option<String>,
    pub target_view: Option<CardView>,
    // Blue or Red
}

impl LuaContext {
    pub async fn new(
        gs: Arc<RwLock<GameState>>,
        actor: &CardView,
        target: Option<CardView>,
        event: String,
        action: String,
    ) -> Self {
        let game_state_guard = gs.read().await;
        let keys: Vec<_> = game_state_guard.players.keys().collect();

        let red_player = game_state_guard.players[keys[0]].clone().read().await.clone();
        let blue_player = game_state_guard.players[keys[1]].clone().read().await.clone();

        let game_state = PrivateGameStateView {
            red_player,
            blue_player,
            turn: game_state_guard.rounds,
        };

        return LuaContext {
            event,
            game_state,
            action_name: action,
            actor_view: actor.clone(),
            actor_id: actor.id.clone(),
            player_turn: game_state_guard.curr_turn.clone(),
            target_id: match &target {
                Some(t) => Some(t.id.clone()),
                None => None,
            },
            target_view: target,
        };
    }

    pub fn to_table(&self, lua: &mlua::Lua) -> Result<mlua::Table, mlua::Error> {
        let context_value = lua.to_value(&self)?;
        return match context_value.as_table() {
            Some(table) => Ok(table.to_owned()),
            None => Err(mlua::Error::BindError),
        }
    }
}
