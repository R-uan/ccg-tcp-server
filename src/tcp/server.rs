use std::collections::HashMap;
use std::{io::Error, net::Ipv4Addr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::RwLock,
};

use super::client::Client;
use crate::game::game::GameInstance;
use crate::models::exit_code::{ExitCode, ExitStatus};
use crate::tcp::client::TemporaryClient;
use crate::tcp::protocol::Protocol;
use crate::{
    logger,
    utils::logger::Logger,
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

/// Represents the main server instance.
///
/// Manages the TCP listener, game state, Lua scripts, connected players, and packet broadcasting.
pub struct ServerInstance {
    pub socket: TcpListener, // The TCP listener for accepting incoming client connections.
    pub listening: Arc<RwLock<bool>>, // Whether the server listen loop is running.
    pub game_instance: Arc<GameInstance>,
    pub exit_status: Arc<RwLock<ExitStatus>>, // The exit status of the server.
    pub players: Arc<RwLock<HashMap<String, Arc<Client>>>>, // A map of connected players, identified by their unique IDs.
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
                    players: Arc::new(RwLock::new(HashMap::new())),
                    exit_status: Arc::new(RwLock::new(ExitStatus::default())),
                })
            }
            Err(error) => Err(error),
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
