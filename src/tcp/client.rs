use super::protocol::{MessageType, Packet, Protocol};
use crate::tcp::server::ServerInstance;
use crate::{
    game::player::Player,
    utils::logger::Logger,
};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::{Arc, LazyLock}
    ,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::RwLock,
};

type ClientState = Arc<RwLock<HashMap<SocketAddr, Arc<Client>>>>;
pub static CLIENTS: LazyLock<ClientState> = LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Represents a connected client in the game server.
///
/// Holds connection state, network streams, and optional player data.
/// All fields are wrapped for safe shared access across async tasks.
pub struct Client {
    pub addr: SocketAddr,
    pub protocol: Arc<Protocol>,
    pub player: Arc<RwLock<Player>>,
    pub connected: Arc<RwLock<bool>>,
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
    pub fn new(read_stream: OwnedReadHalf, write_stream: OwnedWriteHalf, addr: SocketAddr, player: Player, protocol: Arc<Protocol>) -> Self {
        return Self {
            addr,
            protocol,
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
    pub async fn connect(self: Arc<Self>, server: Arc<ServerInstance>) {
        let addr = self.addr;
        let mut buffer = [0; 1024];

        // tokio::spawn({
        //     let self_clone = Arc::clone(&self);
        //     async move { self_clone.tick_game_state().await }
        // });

        let protocol = Protocol::new(server);
        Logger::info(&format!("{addr}: connected"));

        while *self.connected.read().await {
            let mut read_stream_guard = self.read_stream.write().await;
            let bytes_read = match read_stream_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            Logger::info(&format!("{addr}: received {bytes_read} bytes"));
            protocol.handle_incoming(Arc::clone(&self), &buffer[..bytes_read]).await;
        }
    }

    // Continuously receives and forwards game state updates to the client.
    //
    // - Listens on the broadcast receiver for `Packet` messages.
    // - Stops if the client disconnects or if sending fails.
    //
    // Intended to run in its own task while the client is connected.
    // async fn tick_game_state(self: Arc<Self>) {
    //     let transmitter_clone = Arc::clone(&self.protocol.server.transmitter);
    //     let transmitter_guard = transmitter_clone.lock().await;
    //     let mut receiver = transmitter_guard.subscribe();
    //     
    //     drop(transmitter_guard);
    //     
    //     while let Ok(game_state) = receiver.recv().await {
    //         if !*self.connected.read().await {
    //             let mut missed_packets = self.missed_packets.write().await;
    //             missed_packets.push_back(game_state);
    // 
    //             Logger::info(&format!(
    //                 "{}: has {} packets in queue.",
    //                 self.addr,
    //                 missed_packets.len()
    //             ));
    // 
    //             if missed_packets.len() >= 60 {
    //                 missed_packets.pop_back();
    //             }
    //         } else { 
    //             let client = Arc::clone(&self);
    //             self.protocol.send_or_disconnect(client, &game_state).await;
    //         }
    //     }
    // }
}

pub struct TemporaryClient {
    pub addr: SocketAddr,
    pub protocol: Arc<Protocol>,
    pub stream: TcpStream
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
        let mut attempts = 0;
        let mut buffer = [0; 1024];
        while attempts < 3 {
            let bytes = match self.stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break
            };

            match Packet::parse(&buffer[..bytes]) {
                Ok(packet) => {
                    if packet.header.header_type == MessageType::Connect {
                        let self_clone = Arc::new(self);
                        self_clone.protocol.clone().handle_connect(Arc::clone(&self_clone), &packet).await;
                        return;
                    } else {
                        }
                        attempts += 1;
                    }
                Err(_) => { return; }
            }
        }
                        
        let payload = b"Client exceeded connection attempts [3/3]";
        Logger::info(&format!("{}: {}", self.addr, String::from_utf8_lossy(payload)));
        let packet = Packet::new(MessageType::FailedToConnectPlayer, payload);
        let _ = self.stream.write_all(&packet.wrap_packet()).await;
    }
}
