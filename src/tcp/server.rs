use std::{io::Error, net::Ipv4Addr, sync::Arc};
use std::collections::HashMap;
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self, Sender},
        Mutex, RwLock,
    },
    time,
};

use crate::{
    game::{game_state::GameState, script_manager::ScriptManager},
    utils::logger::Logger,
};
use crate::tcp::client::TemporaryClient;
use crate::tcp::protocol::Protocol;
use super::{
    client::{Client, CLIENTS},
    protocol::{MessageType, Packet},
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

pub struct ServerInstance {
    pub socket: TcpListener,
    // pub protocol: Arc<Protocol>,
    pub game_state: Arc<RwLock<GameState>>,
    pub scripts: Arc<RwLock<ScriptManager>>,
    pub transmitter: Arc<Mutex<Sender<Packet>>>,
    pub players: Arc<RwLock<HashMap<String, Arc<Client>>>>,
}

impl ServerInstance {
    /// Creates and binds a new `ServerInstance` to the given port.
    ///
    /// On success, returns an initialized server with a bound TCP listener.
    /// Returns an error if the bind fails.
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        // Lua scripting START
        let mut lua_vm = ScriptManager::new_vm();
        lua_vm.load_scripts()?;
        lua_vm.set_globals().await;

        let scripts = Arc::new(RwLock::new(lua_vm));
        let scripts_clone = Arc::clone(&scripts);
        // Lua scripting END
        
        let game_state = GameState::new_game(scripts_clone);
        let (tx, _) = broadcast::channel::<Packet>(10);
        return match TcpListener::bind((HOST, port)).await {
            Ok(listener) => {
                Logger::debug(&format!("Server listening on port {port}"));
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

        tokio::spawn({
            let protocol_clone = Arc::clone(&protocol);
            async move { protocol_clone.cycle_game_state().await } 
        });

        
        loop {
            if let Ok((client_stream, client_addr)) = self.socket.accept().await {
                Logger::info(&format!("{}: received request", &client_addr));
                let protocol_clone = Arc::clone(&protocol);
                let temp_client = TemporaryClient::new(client_stream, client_addr, protocol_clone).await;
                tokio::spawn(async move {
                    temp_client.handle_temp_client().await;
                });
            }
        }
    }
}
