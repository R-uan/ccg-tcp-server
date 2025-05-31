use std::collections::HashMap;
use std::{io::Error, net::Ipv4Addr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self, Sender},
        Mutex, RwLock,
    },
};

use super::client::Client;
use crate::tcp::client::TemporaryClient;
use crate::tcp::packet::Packet;
use crate::tcp::protocol::Protocol;
use crate::{
    game::{game_state::GameState, script_manager::ScriptManager},
    utils::logger::Logger,
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

/// Represents the main server instance.
///
/// Manages the TCP listener, game state, Lua scripts, connected players, and packet broadcasting.
pub struct ServerInstance {
    /// The TCP listener for accepting incoming client connections.
    pub socket: TcpListener,
    /// The current game state, shared across tasks.
    pub game_state: Arc<RwLock<GameState>>,
    /// The Lua script manager for handling game logic scripts.
    pub scripts: Arc<RwLock<ScriptManager>>,
    /// The transmitter for broadcasting packets to clients.
    pub transmitter: Arc<Mutex<Sender<Packet>>>,
    /// A map of connected players, identified by their unique IDs.
    pub players: Arc<RwLock<HashMap<String, Arc<Client>>>>,
}

impl ServerInstance {
    /// Creates and binds a new `ServerInstance` to the given port.
    ///
    /// - Initializes the Lua script manager and game state.
    /// - Binds the TCP listener to the specified port.
    ///
    /// # Arguments
    /// - `port`: The port number to bind the server to.
    ///
    /// # Returns
    /// - `Ok(ServerInstance)`: If the server is successfully created and bound.
    /// - `Err(Error)`: If the binding fails.
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        // Lua scripting START
        let mut lua_vm = ScriptManager::new_vm();
        lua_vm.load_scripts()?;
        lua_vm.set_globals().await;
        let scripts = Arc::new(RwLock::new(lua_vm));
        // Lua scripting END

        let game_state = GameState::new_game();
        let (tx, _) = broadcast::channel::<Packet>(10);
        return match TcpListener::bind((HOST, port)).await {
            Ok(listener) => {
                Logger::debug(&format!("[SERVER] Listening on port `{port}`"));
                return Ok(ServerInstance {
                    scripts,
                    socket: listener,
                    transmitter: Arc::new(Mutex::new(tx)),
                    game_state: Arc::new(RwLock::new(game_state)),
                    players: Arc::new(RwLock::new(HashMap::new())),
                });
            }
            Err(error) => Err(error),
        };
    }

    /// Starts the main server loop and handles incoming client connections.
    ///
    /// - Spawns a background task to broadcast game state updates.
    /// - Accepts new TCP clients, logs them, registers them, and spawns their handling task.
    ///
    /// Runs indefinitely. Requires `self` as `Arc` for shared access.
    pub async fn listen(self: Arc<Self>) {
        let protocol = Arc::new(Protocol::new(Arc::clone(&self)));

        // Spawn a background task to handle game state updates.
        tokio::spawn({
            let protocol_clone = Arc::clone(&protocol);
            async move { protocol_clone.cycle_game_state().await }
        });

        // Main loop to accept and handle incoming client connections.
        loop {
            if let Ok((stream, addr)) = self.socket.accept().await {
                Logger::info(&format!("[CONNECTION] Accepted request from `{addr}`"));
                let protocol_clone = Arc::clone(&protocol);
                let temp_client = TemporaryClient::new(stream, addr, protocol_clone).await;

                // Spawn a task to handle the temporary client.
                tokio::spawn(async move {
                    temp_client.handle_temp_client().await;
                });
            }
        }
    }
}
