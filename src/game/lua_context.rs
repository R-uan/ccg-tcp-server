use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};

use crate::{
    models::board::{Board, Graveyard},
    utils::logger::Logger,
};

use super::game_state::GameState;

#[derive(Serialize, Clone)]
pub struct GameStateView {
    pub turn: u32,
    pub red_player: PlayerView,
    pub blue_player: PlayerView,
}

#[derive(Serialize, Clone)]
pub struct PlayerView {
    pub id: String,

    pub health: i32,
    pub mana: u32,

    pub hand_size: usize,
    pub deck_size: usize,

    pub board: Board,
    pub graveyard_size: usize,
    pub graveyard: Graveyard,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct CardView {
    pub id: String,
    pub hand_id: u32,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub card_type: String,
    pub effects: Vec<String>,
    pub owner_id: String,
    pub is_exhausted: bool,
    pub position: String,
}

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
        let red_player = gs.red_player.clone().unwrap().read().await.clone();
        let blue_player = gs.blue_player.clone().unwrap().read().await.clone();

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
