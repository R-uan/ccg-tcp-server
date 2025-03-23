use std::{io::Error, net::Ipv4Addr, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        broadcast::{self, Receiver, Sender},
        Mutex,
    },
    time,
};

use crate::{
    game::{
        game_state::GameState,
        player_state::{Player, SHARED_PLAYER_STATE},
    },
    tcp::protocol::{Protocol, ProtocolHeader},
    utils::{checksum::CheckSum, logger::Logger},
};

use super::protocol::ProtocolType;

pub struct ServerInstance {
    pub socket: TcpListener,
    pub game_state: GameState,
}

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

impl ServerInstance {
    ///
    /// Creates a new ServerInstance with:
    /// * A TCPListener bound to 127.0.0.1 and the given port
    /// * A GameState with default initial values
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => {
                Logger::debug(&format!("Server listening on port {port}"));
                Ok(ServerInstance {
                    socket: tcp_stream,
                    game_state: GameState::new_game(),
                })
            }
            Err(error) => Err(error),
        };
    }

    ///
    /// This function does two things:
    /// * Accepts incoming requests from the socket (TCPListener) and sends them to the `handle_client`.
    /// * Fires up the `write_state_update` to periodically send the GameState to connected clients (must have at least
    /// one player connected)
    pub async fn run(self: Arc<Self>) {
        let (tx, _) = broadcast::channel::<Vec<u8>>(10);
        let transmiter = Arc::new(Mutex::new(tx));

        tokio::spawn({
            let server_clone = Arc::clone(&self);
            let tx = Arc::clone(&transmiter);
            async move { ServerInstance::write_state_update(tx, server_clone).await }
        });

        loop {
            let tx = Arc::clone(&transmiter);
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                Logger::info(&format!("{addr}: received request"));
                let tx = tx.lock().await;
                let rx = tx.subscribe();
                tokio::spawn(ServerInstance::handle_client(c_stream, rx));
            }
        }
    }

    ///
    /// Periodically sends the game state connected clients.
    async fn write_state_update(tx: Arc<Mutex<Sender<Vec<u8>>>>, server: Arc<ServerInstance>) {
        let mut interval = time::interval(std::time::Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let players = SHARED_PLAYER_STATE.read().await;

            if players.len() > 0 {
                Logger::info(&format!("Sending game state"));
                let body = b"GameState";
                let game_state = Protocol::create_packet(ProtocolType::GameState, body);
                let tx = tx.lock().await;
                let _ = tx.send(game_state.to_vec());
            }
        }
    }

    async fn handle_client(stream: TcpStream, mut rx: Receiver<Vec<u8>>) {
        let (mut read_stream, write_stream) = stream.into_split();
        let write_stream = Arc::new(Mutex::new(write_stream));

        tokio::spawn({
            let write_stream = Arc::clone(&write_stream);
            async move {
                let mut attempts = 0;
                let mut buffer = [0; 1024];
                let mut player_id: Option<String> = None;
                let addr = read_stream.peer_addr().unwrap();

                loop {
                    let bytes_read = match read_stream.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    Logger::info(&format!("{addr}: received {bytes_read} bytes"));
                    if let Ok(header) = ProtocolHeader::from_bytes(&buffer[..5]) {
                        let payload = &buffer[6..bytes_read];

                        if CheckSum::check(&header.checksum, payload) == false {
                            Logger::error(&format!("{addr}: checksum check failed"));

                            let payload = b"checksum failed";
                            let mut w_stream = write_stream.lock().await;
                            if let Err(_) = Protocol::create_packet(ProtocolType::Err, payload)
                                .send(w_stream, &addr)
                                .await
                            {
                                break;
                            };
                        };

                        match header.ptype {
                            ProtocolType::Connect => {
                                if let Ok(player) = Player::new(payload, &addr) {
                                    player_id = Some(player.id.clone());
                                    Player::add_player(player).await;

                                    let payload = b"player sucessfully connected";
                                    let mut w_stream = write_stream.lock().await;
                                    if let Err(_) = Protocol::create_packet(
                                        ProtocolType::PlayerConnected,
                                        payload,
                                    )
                                    .send(w_stream, &addr)
                                    .await
                                    {
                                        break;
                                    }
                                } else {
                                    Logger::error(&format!("{addr}: unable to connect player"));

                                    let payload = b"unable to connect player";
                                    let mut w_stream = write_stream.lock().await;
                                    if let Err(_) =
                                        Protocol::create_packet(ProtocolType::Err, payload)
                                            .send(w_stream, &addr)
                                            .await
                                    {
                                        break;
                                    }

                                    attempts += 1;
                                }
                            }
                            ProtocolType::Close => {
                                {
                                    let mut w_stream = write_stream.lock().await;
                                    w_stream.write_all(b"0x00").await.unwrap_or_default();
                                }
                                break;
                            }
                            ProtocolType::PlayerMovement => {
                                if let Some(id) = &player_id {
                                    if let Some(player) =
                                        SHARED_PLAYER_STATE.write().await.get_mut(id)
                                    {
                                        player.update_position(payload);
                                    };
                                }
                            }
                            _ => {
                                let payload = b"invalid header";
                                let w_stream = write_stream.lock().await;
                                if let Err(_) = Protocol::create_packet(ProtocolType::Err, payload)
                                    .send(w_stream, &addr)
                                    .await
                                {
                                    break;
                                }
                            }
                        }
                    } else {
                        let payload = b"invalid header";
                        let w_stream = write_stream.lock().await;
                        if let Err(_) = Protocol::create_packet(ProtocolType::Err, payload)
                            .send(w_stream, &addr)
                            .await
                        {
                            break;
                        }
                    }
                }

                Logger::info(&format!("{addr}: closing connection"));
                if let Some(player_id) = player_id {
                    Player::remove_player(&player_id).await;
                }
            }
        });

        tokio::spawn({
            let write_stream = Arc::clone(&write_stream);
            async move {
                while let Ok(game_state) = rx.recv().await {
                    let mut write_guard = write_stream.lock().await; // Lock for writing
                    if write_guard.write_all(&game_state).await.is_err() {
                        break; // Client disconnected
                    }
                    // Write ptype is done, and the lock is released
                }
            }
        });
    }
}
