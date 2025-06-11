use super::client::Client;
use crate::game::game::GameInstance;
use crate::models::exit_code::{ExitCode, ExitStatus};
use crate::tcp::client::TemporaryClient;
use crate::tcp::protocol::Protocol;
use crate::{logger, utils::logger::Logger, SERVER_INSTANCE};
use std::collections::HashMap;
use std::{io::Error, net::Ipv4Addr, sync::Arc};
use tokio::net::TcpStream;
use tokio::{net::TcpListener, sync::RwLock};
use tokio::io::AsyncReadExt;
use crate::models::init_server::InitServerRequest;
use crate::tcp::header::HeaderType;
use crate::tcp::packet::Packet;

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

/// Represents the main server instance.
///
/// Manages the TCP listener, game state, Lua scripts, connected players, and packet broadcasting.
pub struct ServerInstance {
    pub socket: TcpListener, // The TCP listener for accepting incoming client connections.
    pub listening: Arc<RwLock<bool>>, // Whether the server listen loop is running.
    pub game_instance: Arc<GameInstance>,
    pub exit_status: Arc<RwLock<ExitStatus>>, // The exit status of the server.
    pub connected_clients: Arc<RwLock<HashMap<String, Arc<Client>>>>, // A map of connected players, identified by their unique IDs.
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
        let game_instance = GameInstance::create_instance().await?;
        match TcpListener::bind((HOST, port)).await {
            Ok(listener) => {
                logger!(INFO, "[SERVER] Listening on port `{port}`");
                Ok(ServerInstance {
                    socket: listener,
                    game_instance: Arc::new(game_instance),
                    listening: Arc::new(RwLock::new(true)),
                    connected_clients: Arc::new(RwLock::new(HashMap::new())),
                    exit_status: Arc::new(RwLock::new(ExitStatus::default())),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub async fn initialize_server(uninitialized: Arc<UninitializedServer>, request: InitServerRequest) -> Result<ServerInstance, Error> {
        match SERVER_INSTANCE.initialized() {
            true => {
                
            }
            false => {
                if let Ok(server) = Arc::try_unwrap(uninitialized) {
                    
                }
            }
        }
    }
    
    /// Starts the main server loop and handles incoming client connections.
    ///
    /// - Spawns a background task to broadcast game state updates.
    /// - Accepts new TCP clients, logs them, registers them, and spawns their handling task.
    ///
    /// Runs indefinitely. Requires `self` as `Arc` for shared access.
    pub async fn listen(self: Arc<Self>) {
        let protocol = Arc::new(Protocol::new(self.clone(), self.game_instance.clone()));

        // Spawn a background task to handle game state updates.
        // tokio::spawn({
        //     let protocol_clone = Arc::clone(&protocol);
        //     async move { protocol_clone.cycle_game_state().await }
        // });

        // Main loop to accept and handle incoming client connections.
        while *self.listening.read().await {
            match self.socket.accept().await {
                Err(error) => logger!(INFO, "[SERVER] Failed to accept client connection: {error}"),
                Ok((stream, addr)) => {
                    logger!(INFO, "[CONNECTION] Accepted request from `{addr}`");
                    let protocol_clone = Arc::clone(&protocol);

                    // Spawn a task to handle the temporary client.
                    tokio::spawn(async move {
                        let temp_client = TemporaryClient::new(stream, addr, protocol_clone).await;
                        temp_client.handle_temp_client().await;
                    });
                }
            }
        }
    }

    pub async fn close_server(&self, code: ExitCode, reason: &str) {
        let mut exit_status = self.exit_status.write().await;
        exit_status.code = code as i32;
        exit_status.reason = reason.to_string();
        let mut listening = self.listening.write().await;
        *listening = false;
    }
}

pub struct UninitializedServer {
    pub socket: TcpListener,
    pub listening: Arc<RwLock<bool>>,
}

impl UninitializedServer {
    pub async fn create_instance(port: u16) -> Result<Self, Error> {
        match TcpListener::bind((HOST, port)).await {
            Ok(listener) => {
                logger!(INFO, "[SERVER] Listening on port `{port}`");
                Ok(Self {
                    socket: listener,
                    listening: Arc::new(RwLock::new(false))
                })
            }
            Err(error) => Err(error),
        }
    }
    
    pub async fn await_for_initialization(self: Arc<Self>) {
        while *self.listening.read().await {
            match self.socket.accept().await {
                Err(error) => logger!(INFO, "[SERVER] Failed to accept client connection: {error}"),
                Ok((stream, addr)) => {
                    let me = self.clone();
                    tokio::spawn(async move {
                        me.listen_to_connection(stream).await;
                    });
                }
            }
        }
    }

    pub async fn listen_to_connection(self: Arc<Self>, mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        while *self.listening.read().await {
            let read_bytes = match stream.read(&mut buffer).await {
                Ok(0) => return,
                Err(_) => return,
                Ok(n) => n,
            };

            match Packet::parse(&buffer[..read_bytes]) {
                Ok(packet) => {
                    if (packet.header.header_type == HeaderType::InitServer) {
                        match serde_cbor::from_slice::<InitServerRequest>(&packet.payload) {    
                            Err(error) => {}
                            Ok(request) => {
                                
                            }
                        };
                    }
                }
                Err(error) => {}
            }
        }
    }
}
