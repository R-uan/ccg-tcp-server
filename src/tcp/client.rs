use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, LazyLock},
    thread::sleep,
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::{broadcast::Receiver, Mutex, RwLock},
};

use crate::{
    game::player::Player,
    utils::{checksum::CheckSum, errors::NetworkError, logger::Logger},
};

use super::protocol::{self, MessageType, Packet, Protocol};

type ClientState = Arc<RwLock<HashMap<SocketAddr, Arc<Client>>>>;
pub static CLIENTS: LazyLock<ClientState> = LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Represents a connected client in the game server.
///
/// Holds connection state, network streams, and optional player data.
/// All fields are wrapped for safe shared access across async tasks.
pub struct Client {
    /// The client's socket address (IP and port).
    pub addr: SocketAddr,

    /// Whether the client is currently connected.
    /// Wrapped for shared mutable access.
    pub connected: Arc<RwLock<bool>>,

    /// Receiver for incoming broadcast packets.
    pub rx: Arc<Mutex<Receiver<Packet>>>,

    /// Optional player info associated with the client.
    pub player: Arc<RwLock<Option<Player>>>,

    /// Stream used for reading data from the client.
    pub read_stream: Arc<Mutex<OwnedReadHalf>>,

    /// Stream used for writing data to the client.
    pub write_stream: Arc<Mutex<OwnedWriteHalf>>,
}

impl Client {
    /// Creates a new `Client` instance from a TCP stream and address.
    ///
    /// Splits the stream into read/write halves and wraps all fields
    /// in thread-safe containers for async access.
    ///
    /// # Arguments
    /// - `stream`: The TCP stream from the accepted connection.
    /// - `addr`: The client's socket address.
    /// - `rx`: A broadcast receiver for incoming packets.
    ///
    /// # Returns
    /// An `Arc<Client>` ready for use in async tasks.
    pub fn new(stream: TcpStream, addr: SocketAddr, rx: Receiver<Packet>) -> Arc<Self> {
        let (read_stream, write_stream) = stream.into_split();
        return Arc::new(Self {
            addr,
            rx: Arc::new(Mutex::new(rx)),
            player: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(true)),
            read_stream: Arc::new(Mutex::new(read_stream)),
            write_stream: Arc::new(Mutex::new(write_stream)),
        });
    }

    /// Handles the main lifecycle of a connected client.
    ///
    /// - Logs connection and spawns a background game state update task.
    /// - Reads data from the client in a loop, parses packets, and handles them.
    /// - Verifies checksums and sends error responses if validation fails.
    ///
    /// Exits the loop (and drops the client) if the connection is closed or an error occurs.
    pub async fn connect(self: Arc<Self>) {
        let addr = self.addr;
        let mut buffer = [0; 1024];

        tokio::spawn({
            let me = Arc::clone(&self);
            async move { me.tick_game_state().await }
        });

        let client_clone = Arc::clone(&self);
        let protocol = Protocol::new(client_clone);
        Logger::info(&format!("{addr}: connected"));

        while *self.connected.read().await {
            let mut read_stream_guard = self.read_stream.lock().await;
            let bytes_read = match read_stream_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            Logger::info(&format!("{addr}: received {bytes_read} bytes"));
            protocol.handle_incoming(&buffer[..bytes_read]).await;
        }
    }

    /// Gracefully disconnects the client from the server.
    ///
    /// - Logs the disconnection.
    /// - Removes the client from the global `CLIENTS` map.
    /// - Sets its `connected` flag to `false`.
    async fn disconnect(&self) {
        Logger::info(&format!("{}: disconnecting", &self.addr));
        let mut connection_status = self.connected.write().await;
        CLIENTS.write().await.remove(&self.addr);
        *connection_status = false;
    }

    /// Attempts to send a packet to the client, retrying up to 3 times on failure.
    ///
    /// - Serializes the packet and writes it to the client's stream.
    /// - Waits 500ms between retries if sending fails.
    /// - Returns `Err(PackageWriteError)` after 3 failed attempts.
    ///
    /// Logs all outcomes.
    async fn send_packet(&self, packet: &Packet) -> Result<(), NetworkError> {
        let mut tries = 0;
        while tries < 3 {
            let packet_data = packet.wrap_packet();
            let mut stream = self.write_stream.lock().await;

            if stream.write_all(&packet_data).await.is_ok() {
                Logger::info(&format!("{}: packet sent", &self.addr));
                return Ok(());
            }

            Logger::error(&format!(
                "{}: failed to send packet . . . [{}]",
                &self.addr, tries
            ));

            sleep(Duration::from_millis(500));
            tries += 1;
        }

        Logger::error(&format!("{}: failed to send packet", &self.addr));
        return Err(NetworkError::PackageWriteError("unknown error".to_string()));
    }

    /// Sends a packet to the client, disconnecting if the send fails.
    ///
    /// Useful for simplifying repeated send-and-disconnect patterns.
    /// Prevents duplicated error handling logic throughout packet handling.
    pub async fn send_or_disconnect(&self, packet: &Packet) {
        if self.send_packet(packet).await.is_err() {
            self.disconnect().await;
        }
    }

    /// Continuously receives and forwards game state updates to the client.
    ///
    /// - Listens on the broadcast receiver for `Packet` messages.
    /// - Stops if the client disconnects or if sending fails.
    ///
    /// Intended to run in its own task while the client is connected.
    async fn tick_game_state(&self) {
        let mut receiver = self.rx.lock().await;
        let connected = self.connected.read().await;
        while let Ok(game_state) = receiver.recv().await {
            if !*connected {
                break;
            }

            if self.send_packet(&game_state).await.is_err() {
                break;
            };
        }
    }
}
