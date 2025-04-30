use std::{io::Error, net::Ipv4Addr, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self},
        Mutex, RwLock,
    },
};

use crate::{
    game::{self, game_state::GameState, script_manager::ScriptManager},
    utils::logger::Logger,
};

use super::{
    client::{Client, CLIENTS},
    protocol::Packet,
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

pub struct ServerInstance {
    pub socket: TcpListener,
    pub game_state: Arc<RwLock<GameState>>,
    pub scripts: Arc<RwLock<ScriptManager>>,
}

impl ServerInstance {
    /// Creates and binds a new `ServerInstance` to the given port.
    ///
    /// On success, returns an initialized server with a bound TCP listener.
    /// Returns an error if the bind fails.
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        let mut lua_vm = ScriptManager::new_vm();
        lua_vm.load_scripts()?;
        lua_vm.set_globals().await;

        let scripts = Arc::new(RwLock::new(lua_vm));
        let scripts_clone = Arc::clone(&scripts);
        let game_state = GameState::new_game(scripts_clone);
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => {
                Logger::debug(&format!("Server listening on port {port}"));
                return Ok(ServerInstance {
                    scripts,
                    socket: tcp_stream,
                    game_state: Arc::new(RwLock::new(game_state)),
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
    pub async fn listen(&mut self) {
        let (tx, _) = broadcast::channel::<Packet>(10);
        let transmiter = Arc::new(Mutex::new(tx));

        loop {
            let tx = Arc::clone(&transmiter);
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                Logger::info(&format!("{addr}: received request"));
                let tx = tx.lock().await.subscribe();

                let mut clients = CLIENTS.write().await;
                let gs_clone = Arc::clone(&self.game_state);
                let client = Client::new(c_stream, addr, tx, gs_clone);
                clients.insert(addr, Arc::clone(&client));

                tokio::spawn(async move {
                    client.connect().await;
                });
            }

            let clients = CLIENTS.read().await.len();
            if clients == 2 {
                self.create_game_state().await;
            }
        }
    }

    pub async fn create_game_state(&mut self) {
        let clients = CLIENTS.read().await;
        let client_keys: Vec<_> = clients.keys().collect();

        let client0 = &clients[client_keys[0]];
        let client1 = &clients[client_keys[1]];

        let player0_guard = client0.player.read().await;
        let player0 = player0_guard.as_ref().unwrap();
        let blue = Arc::new(player0);

        let player1_guard = client1.player.read().await;
        let player1 = player1_guard.as_ref().unwrap();
        let red = Arc::new(player1);

        let mut game_state = self.game_state.write().await;
        game_state.add_players(blue, red);
    }
}
