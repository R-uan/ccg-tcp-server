use mlua::LuaSerdeExt;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::game::entity::card::CardView;
use super::game_state::{GameState, PrivateGameStateView};

#[derive(Serialize, Clone)]
pub struct LuaContext {
    pub event: String,
    pub action_name: String,

    pub actor_id: String,
    pub actor_view: CardView,
    pub target_id: Option<String>,
    pub target_view: Option<CardView>,
    pub game_state: PrivateGameStateView,
}

impl LuaContext {
    /// Creates a new `LuaContext` instance.
    ///
    /// # Arguments
    /// * `gs` - A thread-safe reference to the current game state.
    /// * `actor` - The `CardView` representing the actor performing the action.
    /// * `target` - An optional `CardView` representing the target of the action.
    /// * `event` - A string describing the event triggering this context.
    /// * `action` - A string describing the action being performed.
    ///
    /// # Returns
    /// A new `LuaContext` instance populated with the provided data and the current game state.
    pub async fn new(
        game_state: Arc<RwLock<GameState>>,
        actor: &CardView,
        target: Option<CardView>,
        event: String,
        action: String,
    ) -> Self {
        let game_state_guard = game_state.read().await;
        let player_views_guard = game_state_guard.player_views.read().await;

        let keys: Vec<_> = player_views_guard.keys().collect();
        let red_player = player_views_guard[keys[0]]
            .clone()
            .read()
            .await
            .clone();
        
        let blue_player = player_views_guard[keys[1]]
            .clone()
            .read()
            .await
            .clone();

        let private_game_state = PrivateGameStateView {
            red_player,
            blue_player,
            turn: game_state_guard.rounds,
        };

        LuaContext {
            event,
            game_state: private_game_state,
            action_name: action,
            actor_view: actor.clone(),
            actor_id: actor.id.clone(),
            target_id: match &target {
                Some(t) => Some(t.id.clone()),
                None => None,
            },
            target_view: target,
        }
    }

    /// Converts the `LuaContext` instance into a Lua table.
    ///
    /// # Arguments
    /// * `lua` - A thread-safe reference to the Lua runtime.
    ///
    /// # Returns
    /// A `Result` containing the Lua table representation of the context or an `mlua::Error` if the conversion fails.
    pub fn to_table(&self, lua: Arc<mlua::Lua>) -> Result<mlua::Table, mlua::Error> {
        let context_value = lua.to_value(&self)?;
        match context_value.as_table() {
            Some(table) => Ok(table.to_owned()),
            None => Err(mlua::Error::BindError),
        }
    }
}
