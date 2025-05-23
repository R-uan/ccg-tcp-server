use super::protocol::{MessageType, Packet, Protocol};
use crate::{game::player::Player, utils::logger::Logger};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    io::AsyncReadExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::RwLock,
};

/// Represents a connected client in the game server.
///
/// Holds connection state, network streams, and optional player data.
/// All fields are wrapped for safe shared access across async tasks.
pub struct Client {
    pub protocol: Arc<Protocol>,
    pub player: Arc<RwLock<Player>>,
    pub connected: Arc<RwLock<bool>>,
    pub addr: Arc<RwLock<SocketAddr>>,
    pub read_stream: Arc<RwLock<OwnedReadHalf>>,
    pub write_stream: Arc<RwLock<OwnedWriteHalf>>,
    pub missed_packets: Arc<RwLock<VecDeque<Packet>>>,
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
    pub fn new(
        read_stream: OwnedReadHalf,
        write_stream: OwnedWriteHalf,
        addr: SocketAddr,
        player: Player,
        protocol: Arc<Protocol>,
    ) -> Self {
        return Self {
            protocol,
            addr: Arc::new(RwLock::new(addr)),
            player: Arc::new(RwLock::new(player)),
            connected: Arc::new(RwLock::new(true)),
            read_stream: Arc::new(RwLock::new(read_stream)),
            write_stream: Arc::new(RwLock::new(write_stream)),
            missed_packets: Arc::new(RwLock::new(VecDeque::new())),
        };
    }

    /// Handles the main lifecycle of a connected client.
    ///
    /// - Logs connection and spawns a background game state update task.
    /// - Reads data from the client in a loop, parses packets, and handles them.
    /// - Verifies checksums and sends error responses if validation fails.
    ///
    /// Exits the loop (and drops the client) if the connection is closed or an error occurs.
    pub async fn connect(self: Arc<Self>) {
        let addr = self.addr.read().await;
        let mut buffer = [0; 1024];

        tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move {
                self_clone.listen_to_game_state().await;
            }
        });

        Logger::info(&format!("[CLIENT] Listening to `{addr}` (Authenticated)"));
        while *self.connected.read().await {
            let mut read_stream_guard = self.read_stream.write().await;
            let bytes_read = match read_stream_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            Logger::info(&format!("[CLIENT] `{addr}` has sent {bytes_read} bytes"));
            self.protocol
                .handle_incoming(Arc::clone(&self), &buffer[..bytes_read])
                .await;
        }
    }

    async fn listen_to_game_state(self: Arc<Self>) {
        let protocol_clone = Arc::clone(&self.protocol);
        let transmitter_clone = Arc::clone(&protocol_clone.server.transmitter);
        let mut receiver = transmitter_clone.lock().await.subscribe();
        while let Ok(game_state) = receiver.recv().await {
            if !*self.connected.read().await {
                let addr = self.addr.read().await;
                let mut missed_packets = self.missed_packets.write().await;
                missed_packets.push_back(game_state);
                Logger::warn(&format!(
                    "[CLIENT] `{addr}` has {} game state packets in queue",
                    &missed_packets.len()
                ));
                continue;
            }

            let write_stream_clone = Arc::clone(&self.write_stream);
            let addr = *Arc::clone(&self.addr).read().await;
            let _ = self
                .protocol
                .send_packet(write_stream_clone, addr, &game_state)
                .await;
        }
    }
}

pub struct TemporaryClient {
    pub addr: SocketAddr,
    pub protocol: Arc<Protocol>,
    pub stream: TcpStream,
}

impl TemporaryClient {
    pub async fn new(stream: TcpStream, addr: SocketAddr, protocol: Arc<Protocol>) -> Self {
        return TemporaryClient {
            addr,
            stream,
            protocol,
        };
    }

    pub async fn handle_temp_client(mut self) {
        let mut buffer = [0; 1024];
        let addr = self.addr.clone();
        Logger::debug(&format!(
            "[CLIENT] Listening to temporary client `{addr}` for authentication"
        ));
        let bytes = match self.stream.read(&mut buffer).await {
            Ok(0) => return,
            Err(_) => return,
            Ok(n) => n,
        };

        match Packet::parse(&buffer[..bytes]) {
            Ok(packet) => {
                if packet.header.header_type == MessageType::Connect {
                    let temp_arc = Arc::new(self);
                    let protocol = Arc::clone(&temp_arc.protocol);
                    if let Err(error) = protocol.handle_connect(temp_arc, &packet).await {
                        Logger::warn(&format!(
                            "[CLIENT] Could not authenticate `{addr}` ({error})"
                        ));
                    };
                } else if packet.header.header_type == MessageType::Reconnect {
                    let temp_arc = Arc::new(self);
                    let protocol = Arc::clone(&temp_arc.protocol);
                    if let Err(error) = protocol.handle_reconnect(temp_arc, &packet).await {
                        Logger::warn(&format!(
                            "[CLIENT] Could not authenticate `{addr}` ({error})"
                        ));
                    };
                }
            }
            Err(error) => {
                Logger::error(&format!("[CLIENT] Invalid packet from `{addr}` ({error})"));
                return;
            }
        }
    }
}
