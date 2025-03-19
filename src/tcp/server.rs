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
    tcp::protocol::{CheckSum, Protocol, ProtocolHeader},
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
                println!("Server connection open: {port}");
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
            let tx = Arc::clone(&transmiter);
            async move { ServerInstance::write_state_update(tx).await }
        });

        loop {
            let tx = Arc::clone(&transmiter);
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                println!("[Incoming] # {addr}");
                let tx = tx.lock().await;
                let rx = tx.subscribe();
                let server_clone = Arc::clone(&self);
                tokio::spawn(ServerInstance::handle_client(server_clone, c_stream, rx));
            }
        }
    }

    ///
    /// Periodically sends the game state connected clients.
    async fn write_state_update(tx: Arc<Mutex<Sender<Vec<u8>>>>) {
        let mut interval = time::interval(std::time::Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let players = SHARED_PLAYER_STATE.read().await;

            if players.len() > 0 {
                println!("[Info] # Sending game state update");
                let body = b"GameState";
                let game_state = Protocol::create_packet(ProtocolType::GameState, body);
                let tx = tx.lock().await;
                let _ = tx.send(game_state.to_vec());
            }
        }
    }

    async fn handle_client(
        server: Arc<ServerInstance>,
        stream: TcpStream,
        mut rx: Receiver<Vec<u8>>,
    ) {
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

                    println!("[Read]# Received {bytes_read} bytes from {addr}");
                    if let Ok(header) = ProtocolHeader::from_bytes(&buffer[..5]) {
                        let payload = &buffer[6..bytes_read];

                        if CheckSum::check(&header.checksum, payload) == false {
                            eprintln!("[Error] # Checksum check failed.");

                            let payload = b"Checksum failed";
                            let packet = Protocol::create_packet(ProtocolType::Err, payload);

                            {
                                let mut w_stream = write_stream.lock().await;
                                if let Err(_) = w_stream.write_all(&packet).await {
                                    eprint!("[Error] # Unable to write to {addr}");
                                    break;
                                }
                            }
                        };

                        match header.operation {
                            ProtocolType::Connect => {
                                if let Ok(player) = Player::new(payload, &addr) {
                                    player_id = Some(player.id.clone());
                                    Player::add_player(player).await;

                                    let payload = b"Player sucessfully connected";
                                    let response = Protocol::create_packet(
                                        ProtocolType::PlayerConnected,
                                        payload,
                                    );

                                    {
                                        let mut w_stream = write_stream.lock().await;
                                        if let Err(_) = w_stream.write_all(&response).await {
                                            eprint!("[Error] # Unable to write to {addr}");
                                            break;
                                        }
                                    }
                                } else {
                                    let payload = b"Unable to connect player";
                                    let e_response =
                                        Protocol::create_packet(ProtocolType::Err, payload);

                                    let mut w_stream = write_stream.lock().await;
                                    if let Err(_) = w_stream.write_all(&e_response).await {
                                        eprint!("[Error] # Unable to write to {addr}");
                                        break;
                                    }

                                    attempts += 1;
                                    eprint!(
                                        "[Error] # Unable to connect {addr}...Attempts: {attempts}"
                                    );
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
                                let e_body = b"Invalid header";
                                let e_response = Protocol::create_packet(ProtocolType::Err, e_body);
                                {
                                    let mut w_stream = write_stream.lock().await;
                                    if let Err(_) = w_stream.write_all(&e_response).await {
                                        eprint!("[Error] # Unable to write to {addr}");
                                        break;
                                    };
                                }
                            }
                        }
                    } else {
                        let e_body = b"Invalid header";
                        let e_repose = Protocol::create_packet(ProtocolType::Err, e_body);
                        {
                            let mut w_stream = write_stream.lock().await;
                            if let Err(_) = w_stream.write_all(&e_repose).await {
                                eprint!("[Error] # Unable to write to {addr}");
                                break;
                            };
                        }
                    }
                }

                println!("[Close] # Closing connection with {addr}");
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
                    // Write operation is done, and the lock is released
                }
            }
        });
    }
}
