use super::protocol::Protocol;
use crate::game::entity::player::Player;
use crate::tcp::header::HeaderType;
use crate::tcp::packet::Packet;
use crate::{logger, utils::logger::Logger};
use std::{collections::VecDeque, net::SocketAddr, sync::Arc};
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
        protocol: Arc<Protocol>,
        player: Arc<RwLock<Player>>,
    ) -> Self {
        Self {
            player,
            protocol,
            addr: Arc::new(RwLock::new(addr)),
            connected: Arc::new(RwLock::new(true)),
            read_stream: Arc::new(RwLock::new(read_stream)),
            write_stream: Arc::new(RwLock::new(write_stream)),
            missed_packets: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Handles the main lifecycle of a connected client.
    ///
    /// - Logs connection and spawns a background game state update task.
    /// - Reads data from the client in a loop, parses packets, and handles them.
    /// - Verifies checksums and sends error responses if validation fails.
    ///
    /// Exits the loop (and drops the client) if the connection is closed, or an error occurs.
    pub async fn connect(self: Arc<Self>) {
        let addr = self.addr.read().await;
        logger!(DEBUG, "[CLIENT] Listening to `{addr}` (Authenticated)");

        tokio::spawn({
            let self_clone = Arc::clone(&self);
            async move {
                self_clone.listen_to_game_state().await;
            }
        });

        let mut buffer = [0; 1024];
        while *self.connected.read().await {
            let mut read_stream_guard = self.read_stream.write().await;
            let bytes_read = match read_stream_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            self.protocol
                .handle_incoming(Arc::clone(&self), &buffer[..bytes_read])
                .await;
        }
    }

    /// Listens to game state updates and sends them to the client.
    ///
    /// - If the client is disconnected, queues the game state packets.
    /// - Sends missed packets if any are queued.
    /// - Sends the current game state to the client.
    ///
    /// This function runs in a loop and exits when the receiver is dropped.
    async fn listen_to_game_state(self: Arc<Self>) {
        let protocol_clone = Arc::clone(&self.protocol);
        let transmitter_clone = Arc::clone(&protocol_clone.transmitter);
        let mut receiver = transmitter_clone.lock().await.subscribe();
        while let Ok(game_state) = receiver.recv().await {
            if !*self.connected.read().await {
                let addr = self.addr.read().await;
                let mut missed_packets = self.missed_packets.write().await;
                missed_packets.push_back(game_state);

                if missed_packets.len() > 30 {
                    missed_packets.pop_front();
                }

                logger!(
                    WARN,
                    "[CLIENT] `{addr}` has {} game state packets in queue",
                    &missed_packets.len()
                );

                continue;
            }

            if self.missed_packets.read().await.len() > 0 {
                let client_clone = Arc::clone(&self);
                self.protocol.send_missed_packets(client_clone).await;
            }

            let client_clone = Arc::clone(&self);
            let _ = self.protocol.send_packet(client_clone, &game_state).await;
        }
    }

    /// Reconnects a client using a temporary client instance.
    ///
    /// - Updates the client's read/write streams, address, and connection status.
    ///
    /// # Arguments
    /// - `temporary_client`: A `TemporaryClient` instance containing the new connection details.
    pub async fn reconnect(self: Arc<Self>, temporary_client: TemporaryClient) {
        let (read, write) = temporary_client.stream.into_split();

        let mut write_stream = self.write_stream.write().await;
        let mut read_stream = self.read_stream.write().await;
        let mut addr = self.addr.write().await;
        let mut connected = self.connected.write().await;

        *write_stream = write;
        *read_stream = read;
        *addr = temporary_client.addr;
        *connected = true;
    }
}

/// Represents a temporary client used during the authentication or reconnection process.
///
/// This struct holds the necessary information for handling a temporary client connection,
/// such as the client's socket address, the TCP stream, and the protocol instance.
///
/// Temporary clients are used to authenticate new connections or handle reconnection requests
/// before they are fully integrated into the main client management system.
pub struct TemporaryClient {
    /// The socket address of the temporary client.
    pub addr: SocketAddr,
    /// The protocol instance used to handle communication with the client.
    pub protocol: Arc<Protocol>,
    /// The TCP stream associated with the temporary client.
    pub stream: TcpStream,
}

impl TemporaryClient {
    /// Creates a new `TemporaryClient` instance.
    ///
    /// # Arguments
    /// - `stream`: The TCP stream for the temporary client.
    /// - `addr`: The socket address of the temporary client.
    /// - `protocol`: The protocol instance to handle client communication.
    ///
    /// # Returns
    /// A new `TemporaryClient` instance.
    pub async fn new(stream: TcpStream, addr: SocketAddr, protocol: Arc<Protocol>) -> Self {
        TemporaryClient {
            addr,
            stream,
            protocol,
        }
    }

    /// Handles the lifecycle of a temporary client.
    ///
    /// - Reads data from the client for authentication.
    /// - Parses the packet and determines if it's a `Connect` or `Reconnect` request.
    /// - Calls the appropriate protocol handler for authentication.
    ///
    /// Exits if the client sends invalid data or an error occurs.
    pub async fn handle_temp_client(mut self) {
        let mut buffer = [0; 1024];
        let addr = self.addr.clone();
        logger!(
            DEBUG,
            "[CLIENT] Listening to temporary client `{addr}` for authentication"
        );

        loop {
            let bytes = match self.stream.read(&mut buffer).await {
                Ok(0) => return,
                Err(_) => return,
                Ok(n) => n,
            };

            match Packet::parse(&buffer[..bytes]) {
                Ok(packet) => {
                    if packet.header.header_type == HeaderType::Connect {
                        let temp_arc = Arc::new(self);
                        let protocol = Arc::clone(&temp_arc.protocol);
                        if let Err(error) = protocol.handle_connect(temp_arc, &packet).await {
                            logger!(ERROR, "[CLIENT] Could not authenticate `{addr}` ({error})");
                        };
                        break;
                    } else if packet.header.header_type == HeaderType::Reconnect {
                        let temp_arc = Arc::new(self);
                        let protocol = Arc::clone(&temp_arc.protocol);
                        if let Err(error) = protocol.handle_reconnect(temp_arc, &packet).await {
                            logger!(ERROR, "[CLIENT] Could not authenticate `{addr}` ({error})");
                        } else {
                            logger!(INFO, "[CLIENT] `{addr}` has been reconnected as `todo`")
                        }
                        break;
                    }
                }
                Err(error) => {
                    logger!(ERROR, "[CLIENT] Invalid packet from `{addr}` ({error})");
                    return;
                }
            }
        }
    }
}
