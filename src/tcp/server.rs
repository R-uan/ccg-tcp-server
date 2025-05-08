use std::{io::Error, net::Ipv4Addr, sync::Arc};

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

use super::{
    client::{Client, CLIENTS},
    protocol::{MessageType, Packet},
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

pub struct ServerInstance {
    pub socket: TcpListener,
    pub game_state: Arc<RwLock<GameState>>,
    pub scripts: Arc<RwLock<ScriptManager>>,
    pub transmiter: Arc<Mutex<Sender<Packet>>>,
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
        let (tx, _) = broadcast::channel::<Packet>(10);
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => {
                Logger::debug(&format!("Server listening on port {port}"));
                return Ok(ServerInstance {
                    scripts,
                    socket: tcp_stream,
                    transmiter: Arc::new(Mutex::new(tx)),
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
        loop {
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                let tx = Arc::clone(&self.transmiter);
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
        {
            let clients = CLIENTS.read().await;
            let client_keys: Vec<_> = clients.keys().collect();

            let client0 = &clients[client_keys[0]];
            let client1 = &clients[client_keys[1]];

            let player0_guard = client0.player.read().await;
            let player1_guard = client1.player.read().await;

            let player1 = player1_guard.as_ref().unwrap();
            let player0 = player0_guard.as_ref().unwrap();

            let mut game_state = self.game_state.write().await;

            let red;
            let blue;

            if &player0.player_color == "blue" {
                blue = Arc::new(player0);
                red = Arc::new(player1);
            } else {
                blue = Arc::new(player1);
                red = Arc::new(player0);
            }

            game_state.add_players(blue, red);
        }

        self.get_cards().await;
    }

    pub async fn get_cards(&self) {
        let clients = CLIENTS.read().await;
        let client_keys: Vec<_> = clients.keys().collect();

        let client0 = &clients[client_keys[0]];
        let client1 = &clients[client_keys[1]];

        let player0_guard = client0.player.read().await;
        let player0 = player0_guard.as_ref().unwrap();

        let player1_guard = client1.player.read().await;
        let player1 = player1_guard.as_ref().unwrap();

        let mut cards: Vec<&str> = vec![];

        for card in &player0.current_deck.cards {
            cards.push(&card.id);
        }

        for card in &player1.current_deck.cards {
            cards.push(&card.id);
        }

        let mut game_state = self.game_state.write().await;
        game_state.fetch_cards_details(cards).await;
    }

    pub async fn write_state_update(tx: Arc<Mutex<Sender<Packet>>>) {
        let mut interval = time::interval(std::time::Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let clients = CLIENTS.read().await;
            if clients.len() > 0 {
                Logger::info(&"Sending game state".to_string());
                let packet = Packet::new(MessageType::GAMESTATE, b"pretend");
                let tx = tx.lock().await;
                let _ = tx.send(packet);
            }
        }
    }
}
