use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader, Error},
    path::PathBuf,
    sync::Arc,
};

use crate::game::lua_context::LuaContext;
use crate::models::game_action::GameAction;
use crate::utils::errors::GameLogicError;
use crate::utils::logger::Logger;
use mlua::{Function, Lua, LuaSerdeExt, Value};
use tokio::sync::Mutex;

pub struct ScriptManager {
    pub lua: Arc<Lua>,                              // Shared Lua VM instance
    pub core: Mutex<HashMap<String, Function>>,     // Core script functions
    pub cards: Mutex<HashMap<String, Function>>,    // Card-related script functions
    pub effects: Mutex<HashMap<String, Function>>,  // Effect-related script functions
    pub triggers: Mutex<HashMap<String, Function>>, // Trigger-related script functions
}

impl ScriptManager {
    /// Creates a new instance of `ScriptManager` with an initialized Lua VM and empty function maps.
    pub fn new_vm() -> Self {
        let lua = Lua::new();
        return Self {
            lua: Arc::new(lua),
            core: Mutex::new(HashMap::new()),
            cards: Mutex::new(HashMap::new()),
            effects: Mutex::new(HashMap::new()),
            triggers: Mutex::new(HashMap::new()),
        };
    }

    /// Loads Lua scripts from the `./scripts` directory into the Lua VM.
    /// Only directories named "core", "cards", "effects", or "triggers" are processed.
    pub fn load_scripts(&mut self) -> Result<(), Error> {
        let folders = vec!["core", "cards", "effects", "triggers"];
        for entry in fs::read_dir("./scripts")? {
            let path = entry?.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap();
                if folders.contains(&name) {
                    Logger::debug(&format!("[SCRIPTS] Reading from: `{name}` directory"));
                    let _ = self.load_file(&path);
                }
            }
        }

        return Ok(());
    }

    /// Loads individual Lua files from a given directory into the Lua VM.
    /// Logs errors if a file cannot be read or executed.
    fn load_file(&self, dir: &PathBuf) -> Result<(), Error> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension() == Some(OsStr::new("lua")) {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                match fs::read_to_string(&path) {
                    Ok(code) => {
                        Logger::debug(&format!("[SCRIPTS] Loading script: `{name}`"));
                        let _ = self.lua.load(&code).exec();
                    }
                    Err(e) => {
                        let error = e.to_string();
                        Logger::error(&format!("[SCRIPTS] Couldn't load file `{name}`: {error}"));
                    }
                }
            }
        }

        Ok(())
    }

    /// Sets global Lua functions into categorized maps (`core`, `cards`, `effects`, `triggers`).
    /// Reads function names from `.txt` files in the `./scripts` directory.
    pub(crate) async fn set_globals(&mut self) {
        let globals = self.lua.globals();
        if let Ok(files) = fs::read_dir("./scripts") {
            for entry in files {
                let path = entry.unwrap().path();
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                if path.extension() == Some(OsStr::new("txt")) {
                    let file = fs::File::open(path).unwrap();
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        let func_name = line.unwrap();
                        match globals.get::<Function>(func_name.to_owned()) {
                            Ok(function) => {
                                if file_name.contains("core") {
                                    Logger::debug(&format!(
                                        "[CORE] Setting function into map `{func_name}`"
                                    ));
                                    let mut core_guard = self.core.lock().await;
                                    core_guard.insert(func_name, function);
                                } else if file_name.contains("card") {
                                    Logger::debug(&format!(
                                        "[SCRIPTS] [CARDS] Setting function into map `{func_name}`"
                                    ));
                                    let mut card_guard = self.cards.lock().await;
                                    card_guard.insert(func_name, function);
                                } else if file_name.contains("effect") {
                                    Logger::debug(&format!(
                                        "[SCRIPTS] [EFFECTS] Setting function into map `{func_name}`"
                                    ));
                                    let mut effects_guard = self.effects.lock().await;
                                    effects_guard.insert(func_name, function);
                                } else if file_name.contains("trigger") {
                                    Logger::debug(&format!(
                                        "[SCRIPTS] [TRIGGERS] Setting function into map `{func_name}`"
                                    ));
                                    let mut triggers_guard = self.triggers.lock().await;
                                    triggers_guard.insert(func_name, function);
                                }
                            }
                            Err(e) => {
                                let error = e.to_string();
                                Logger::error(&format!(
                                    "[SCRIPTS] Unable to set function `{func_name}` ({error})"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Retrieves a Lua function from the appropriate map based on the action prefix.
    /// The action format is expected to be `<category>:<function_name>`.
    pub async fn get_function(&self, action: &str) -> Option<Function> {
        let action_parts: Vec<&str> = action.splitn(2, ":").collect();
        return match action_parts.as_slice() {
            ["cards", key] => self.cards.lock().await.get(*key).cloned(),
            ["core", key] => self.core.lock().await.get(*key).cloned(),
            ["effects", key] => self.effects.lock().await.get(*key).cloned(),
            ["triggers", key] => self.triggers.lock().await.get(*key).cloned(),
            _ => None,
        };
    }

    /// Calls a Lua function by its action name and returns a list of `GameAction` results.
    /// Returns an error if the function is not callable or the result is invalid.
    pub async fn call_function(&self, action: &str) -> Result<Vec<GameAction>, GameLogicError> {
        if let Some(function) = self.get_function(action).await {
            let lua_value: Value = function
                .call("")
                .map_err(|_| GameLogicError::FunctionNotCallable(action.to_string()))?;
            let game_actions: Vec<GameAction> = self
                .lua
                .from_value(lua_value)
                .map_err(|_| GameLogicError::InvalidGameActions)?;
            return Ok(game_actions);
        }

        return Err(GameLogicError::FunctionNotFound(
            action.to_string(),
            "None".to_string(),
        ));
    }

    /// Calls a Lua function with a `LuaContext` and returns a list of `GameAction` results.
    /// Returns an error if the function is not callable or the result is invalid.
    pub async fn call_function_ctx(
        &self,
        action: &str,
        ctx: LuaContext,
    ) -> Result<Vec<GameAction>, GameLogicError> {
        let lua_table = ctx.to_table(self.lua.clone());
        if let Some(function) = self.get_function(action).await {
            let lua_value: Value = function
                .call(lua_table)
                .map_err(|_| GameLogicError::FunctionNotCallable(action.to_string()))?;
            let game_actions: Vec<GameAction> = self
                .lua
                .from_value(lua_value)
                .map_err(|_| GameLogicError::InvalidGameActions)?;
            return Ok(game_actions);
        }

        return Err(GameLogicError::FunctionNotFound(
            action.to_string(),
            ctx.actor_id.to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_function() {
        let mut script_manager = ScriptManager::new_vm();
        let load_scripts = script_manager.load_scripts();
        assert!(load_scripts.is_ok());
        script_manager.set_globals().await;
        let function = script_manager.get_function("core:test").await;
        assert!(function.is_some());
    }

    #[tokio::test]
    async fn test_call_function() {
        let mut sm = ScriptManager::new_vm();
        let load_scripts = sm.load_scripts();
        assert!(load_scripts.is_ok());
        sm.set_globals().await;
        let function = sm.call_function("core:test").await;
        assert!(function.is_ok());
        if let Ok(actions) = function {
            assert_eq!(2, actions.len());
        }
    }
}
