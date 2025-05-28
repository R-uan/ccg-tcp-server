use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader, Error},
    path::PathBuf,
    sync::Arc,
};

use mlua::{Function, Lua};
use tokio::sync::Mutex;

use crate::utils::logger::Logger;

pub struct ScriptManager {
    pub lua: Arc<Lua>,
    pub core: Mutex<HashMap<String, Function>>,
    pub cards: Mutex<HashMap<String, Function>>,
    pub effects: Mutex<HashMap<String, Function>>,
    pub triggers: Mutex<HashMap<String, Function>>,
}

impl ScriptManager {
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

    pub async fn get_function(&self, action: &str) -> Option<Function> {
        let action_parts: Vec<&str> = action.splitn(2, ":").collect();
        return match action_parts.as_slice() {
            ["cards", key] => self.cards.lock().await.get(*key).cloned(),
            ["core", key] => self.core.lock().await.get(*key).cloned(),
            ["effects", key] => self.effects.lock().await.get(*key).cloned(),
            ["triggers", key] => self.triggers.lock().await.get(*key).cloned(),
            _ => None
        };
    }
}
