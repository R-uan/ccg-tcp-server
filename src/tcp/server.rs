use super::client::Client;
use crate::game::game::GameInstance;
use crate::models::exit_code::ExitStatus;
use crate::models::init_server::InitServerRequest;
use crate::tcp::client::TemporaryClient;
use crate::tcp::header::HeaderType;
use crate::tcp::packet::Packet;
use crate::tcp::protocol::Protocol;
use crate::utils::errors::ServerInstanceError;
use crate::{logger, utils::logger::Logger, SERVER_INSTANCE};
use std::collections::HashMap;
use std::{io::Error, net::Ipv4Addr, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::{net::TcpListener, sync::RwLock};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

/// Represents the main server instance.
///
/// Manages the TCP listener, game state, Lua scripts, connected players, and packet broadcasting.
pub struct ServerInstance {
    pub socket: TcpListener, // The TCP listener for accepting incoming client connections.
    pub listening: Arc<RwLock<bool>>, // Whether the server listen loop is running.
    pub game_instance: Arc<GameInstance>,
    pub exit_status: Arc<RwLock<Option<ExitStatus>>>, // The exit status of the server.
    pub connected_clients: Arc<RwLock<HashMap<String, Arc<Client>>>>, // A map of connected players, identified by their unique IDs.
}

impl ServerInstance {
    pub async fn init_server(
        uninitialized: Arc<UninitializedServer>,
        request: InitServerRequest,
    ) -> Result<ServerInstance, ServerInstanceError> {
        match SERVER_INSTANCE.initialized() {
            true => Err(ServerInstanceError::AlreadyInitialized),
            false => {
                if let Ok(server) = Arc::try_unwrap(uninitialized) {
                    match GameInstance::create_instance(request.players).await {
                        Ok(game_instance) => Ok(ServerInstance {
                            socket: server.socket,
                            game_instance: Arc::new(game_instance),
                            exit_status: Arc::new(RwLock::new(None)),
                            listening: Arc::new(RwLock::new(false)),
                            connected_clients: Arc::new(RwLock::new(HashMap::new())),
                        }),
                        Err(error) => Err(ServerInstanceError::GameInstanceFail(error.to_string())),
                    }
                } else {
                    Err(ServerInstanceError::UnwrapFailed)
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
                    listening: Arc::new(RwLock::new(false)),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub async fn await_for_initialization(
        self: Arc<Self>,
    ) -> Result<ServerInstance, ServerInstanceError> {
        while *self.listening.read().await {
            return match self.socket.accept().await {
                Err(error) => {
                    logger!(INFO, "[SERVER] Failed to accept client connection: {error}");
                    Err(ServerInstanceError::PlaceHolderError)
                }
                Ok((stream, _)) => {
                    let me = self.clone();
                    me.listen_to_connection(stream).await
                }
            }
        }
        
        Err(ServerInstanceError::PlaceHolderError)
    }

    pub async fn listen_to_connection(
        self: Arc<Self>,
        mut stream: TcpStream,
    ) -> Result<ServerInstance, ServerInstanceError> {
        let mut buffer = [0; 1024];
        while *self.listening.read().await {
            let read_bytes = match stream.read(&mut buffer).await {
                Ok(0) => return Err(ServerInstanceError::PlaceHolderError),
                Err(_) => return Err(ServerInstanceError::PlaceHolderError),
                Ok(n) => n,
            };

            let mut send_packet = async |packet: Packet| {
                let _ = stream.write(&packet.wrap_packet()).await;
            };

            match Packet::parse(&buffer[..read_bytes]) {
                Ok(packet) => {
                    if packet.header.header_type == HeaderType::InitServer {
                        return match serde_cbor::from_slice::<InitServerRequest>(&packet.payload) {
                            Err(error) => {
                                let packet =
                                    Packet::new(HeaderType::ERROR, error.to_string().as_bytes());
                                send_packet(packet).await;
                                Err(ServerInstanceError::PlaceHolderError)
                            }
                            Ok(request) => {
                                match ServerInstance::init_server(self.clone(), request).await {
                                    Ok(server) => Ok(server),
                                    Err(error) => {
                                        let packet = Packet::new(
                                            HeaderType::ERROR,
                                            error.to_string().as_bytes(),
                                        );
                                        send_packet(packet).await;
                                        Err(ServerInstanceError::PlaceHolderError)
                                    }
                                }
                            }
                        };
                    }
                }
                Err(error) => {
                    let packet = Packet::new(HeaderType::ERROR, error.to_string().as_bytes());
                    send_packet(packet).await;
                }
            }
        }

        Err(ServerInstanceError::PlaceHolderError)
    }
}
