use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, LazyLock},
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
    utils::{checksum::CheckSum, errors::PackageWriteError, logger::Logger},
};

use super::protocol::{MessageType, Packet};

type ClientState = Arc<RwLock<HashMap<SocketAddr, Arc<Client>>>>;
pub static CLIENTS: LazyLock<ClientState> = LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct Client {
    pub addr: SocketAddr,
    pub connected: Arc<RwLock<bool>>,
    pub rx: Arc<Mutex<Receiver<Packet>>>,
    pub player: Arc<RwLock<Option<Player>>>,
    pub read_stream: Arc<Mutex<OwnedReadHalf>>,
    pub write_stream: Arc<Mutex<OwnedWriteHalf>>,
}

impl Client {
    /**
        Creates a Arc<Client> instance. \
        The TcpStream is split into `read_stream` and `write_stream`
    */
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

    /**
       Connects and listens to the client's incoming packets while parsing the packet into the
       Packet structure and validating the checksum.
       * If the packet is valid, then it's passed to the `handle_packet` function where it will
       be properly handled.
       * If the packet is not valid or the checksum is not valid, then send back an error response
       through the stream.
    */
    pub async fn connect(self: Arc<Self>) {
        let addr = self.addr;
        let mut buffer = [0; 1024];
        let connected = self.connected.read().await;

        Logger::info(&format!("{addr}: connected"));

        tokio::spawn({
            let me = Arc::clone(&self);
            async move { me.game_state().await }
        });

        while *connected {
            let mut read_stream_guard = self.read_stream.lock().await;
            let bytes_read = match read_stream_guard.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            Logger::info(&format!("{addr}: received {bytes_read} bytes"));
            if let Ok(packet) = Packet::parse(&buffer[..bytes_read]) {
                Logger::info(&format!("{addr}: packet sucessfuly parsed"));
                if !CheckSum::check(&packet.header.checksum, &packet.payload) {
                    Logger::error(&format!("{addr}: checksum check failed"));
                    let packet = Packet::new(MessageType::INVALIDCHECKSUM, b"");
                    if self.send_packet(&packet).await.is_err() {
                        break;
                    };
                }

                self.handle_packet(&packet).await
            } else {
                Logger::info(&format!("{addr}: packet couldn't be parsed"));
            }
        }
    }

    /**
        Mutates self.connected status to false, stopping all loops that utilizes it as a condition.
        It also removes Self (Client) instance from the global client HashMap.
    */
    async fn disconnect(&self) {
        Logger::info(&format!("{}: disconnecting", &self.addr));
        let mut connection_status = self.connected.write().await;
        CLIENTS.write().await.remove(&self.addr);
        *connection_status = false;
    }

    /**
       Sends given packet through the client's `write_stream`.
    */
    async fn send_packet(&self, packet: &Packet) -> Result<(), PackageWriteError> {
        let packet_data = packet.wrap_packet();
        let mut stream = self.write_stream.lock().await;

        if stream.write_all(&packet_data).await.is_err() {
            Logger::error(&format!("{}: failed to send packet", &self.addr));
            return Err(PackageWriteError);
        }

        return Ok(());
    }

    /**
        Handles incoming packet based on the header's `MessageType` \
        This function can disconnect the client.
    */
    async fn handle_packet(&self, packet: &Packet) {
        let message_type = &packet.header.header_type;
        match message_type {
            MessageType::CONNECT => {
                let mut player_guard = self.player.write().await;
                if player_guard.is_some() {
                    Logger::warn(&format!("{}: player already connected", &self.addr));
                    let payload = b"this stream already has a client attached to it";
                    let packet = Packet::new(MessageType::ALREADYCONNECTED, payload);
                    if self.send_packet(&packet).await.is_err() {
                        self.disconnect().await;
                    };
                }
                if let Ok(player) = Player::new(&packet.payload) {
                    Logger::info(&format!(
                        "{}: player connected [{}]",
                        &self.addr, &player.uuid
                    ));
                    *player_guard = Some(player);
                    let payload = b"yipee, player connected";
                    let packet = Packet::new(MessageType::CONNECT, payload);
                    if self.send_packet(&packet).await.is_err() {
                        self.disconnect().await;
                    }
                } else {
                    Logger::info(&format!("{}: invalid player data", &self.addr));
                    let packet = Packet::new(MessageType::INVALIDPLAYERDATA, b"");
                    if self.send_packet(&packet).await.is_err() {
                        self.disconnect().await;
                    };
                }
            }
            MessageType::DISCONNECT => {
                Logger::warn(&format!("{}: client disconnecting", &self.addr));
                let packet = Packet::new(MessageType::DISCONNECT, b"");
                if self.send_packet(&packet).await.is_err() {
                    self.disconnect().await;
                };
            }
            _ => {
                Logger::warn(&format!("{}: invalid header", &self.addr));
                let packet = Packet::new(MessageType::INVALIDHEADER, b"");
                if self.send_packet(&packet).await.is_err() {
                    self.disconnect().await;
                };
            }
        }
    }

    /**
        Receives and sends the game state packet through the client's `write_stream`.
    */
    async fn game_state(&self) {
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
