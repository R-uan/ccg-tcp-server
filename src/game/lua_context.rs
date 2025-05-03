use mlua::LuaSerdeExt;
use serde::Serialize;

use crate::{
    models::views::{CardView, GameStateView},
    utils::logger::Logger,
};

use super::game_state::GameState;

#[derive(Serialize, Clone)]
pub struct LuaContext {
    pub event: String,
    pub player_turn: String,
    pub action_name: String,

    pub actor_id: String,
    pub actor_view: CardView,
    pub game_state: GameStateView,
    pub target_id: Option<String>,
    pub target_view: Option<CardView>,
    // Blue or Red
}

impl LuaContext {
    pub async fn new(
        gs: &GameState,
        actor: &CardView,
        target: Option<CardView>,
        event: String,
        action: String,
    ) -> Self {
        let keys: Vec<_> = gs.players.keys().collect();

        let red_player = gs.players[keys[0]].clone().read().await.clone();
        let blue_player = gs.players[keys[1]].clone().read().await.clone();

        let game_state = GameStateView {
            red_player,
            blue_player,
            turn: gs.rounds,
        };

        return LuaContext {
            event,
            game_state,
            action_name: action,
            actor_view: actor.to_owned(),
            actor_id: actor.id.to_owned(),
            player_turn: gs.curr_turn.to_owned(),
            target_id: match &target {
                Some(t) => Some(t.id.to_owned()),
                None => None,
            },
            target_view: target,
        };
    }

    pub fn to_table(&self, lua: &mlua::Lua) -> Result<mlua::Table, mlua::Error> {
        let context_value = lua.to_value(&self)?;
        Logger::info("Works I guess ?");
        match context_value.as_table() {
            Some(table) => return Ok(table.to_owned()),
            None => return Err(mlua::Error::BindError),
        }
    }
}
