use mlua::LuaSerdeExt;
use serde::Serialize;

use crate::models::board::{Board, Graveyard};

#[derive(Serialize, Clone)]
pub struct LuaContext {
    pub actor_id: String,
    pub target_id: Option<String>,

    pub event: String,
    pub action_name: String,

    pub game_state: GameStateView,
    pub actor_view: CardView,
    pub target_view: Option<CardView>,

    // Blue or Red
    pub turn_player: String,
}
#[derive(Serialize, Clone)]
pub struct GameStateView {
    pub turn: u32,
    pub red_player: PlayerView,
    pub blue_player: PlayerView,
    pub battlefield: Vec<CardView>,
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

#[derive(Serialize, Clone)]
pub struct CardView {
    pub id: String,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub effects: Vec<String>,
    pub owner_id: String,
    pub is_exhausted: bool,
    pub position: String,
}

impl LuaContext {
    pub fn to_table(&self, lua: &mlua::Lua) -> Result<mlua::Table, mlua::Error> {
        let context_value = lua.to_value(&self)?;
        match context_value.as_table() {
            Some(table) => return Ok(table.to_owned()),
            None => return Err(mlua::Error::BindError),
        }
    }
}
