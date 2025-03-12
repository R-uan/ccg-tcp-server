use std::{
    collections::HashMap,
    io::Error,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    game::{game_state::GameState, player_state::PlayerState},
    tcp::protocol::{PacketHeader, Protocol},
};

use super::protocol::HeaderTypes;

pub struct ServerInstance {
    pub server_port: u16,
    pub socket: TcpListener,
    pub game_state: GameState,
    pub player_state: Arc<RwLock<HashMap<String, PlayerState>>>,
}

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

impl ServerInstance {
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => {
                println!("Server connection open: {port}");
                Ok(ServerInstance {
                    server_port: port,
                    socket: tcp_stream,
                    game_state: GameState::new_game(),
                    player_state: Arc::new(RwLock::new(HashMap::new())),
                })
            }
            Err(error) => Err(error),
        };
    }

    async fn handle_client(server: Arc<ServerInstance>, mut c_stream: TcpStream, addr: SocketAddr) {
        let mut attempts = 0;
        let mut buffer = [0; 1024];
        let mut player_id: Option<String> = None;

        loop {
            let bytes_read = match c_stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            println!("[Read]# Received {bytes_read} bytes from {addr}");
            if let Ok(header) = PacketHeader::from_bytes(&buffer[..5]) {
                match header.message_type {
                    HeaderTypes::Connect => {
                        if let Ok(player) = PlayerState::new(&buffer[6..bytes_read - 1]) {
                            player_id = Some(player.id.clone());
                            server.add_player(player).await;

                            let body = b"Player sucessfully connected";

                            let e_response =
                                Protocol::create_packet(HeaderTypes::PlayerConnected, body);

                            if let Err(_) = c_stream.write_all(&e_response).await {
                                eprint!("[Error] # Unable to write to {addr}");
                                break;
                            }
                        } else {
                            let body = b"Unable to connect player";

                            let e_response = Protocol::create_packet(HeaderTypes::Err, body);

                            if let Err(_) = c_stream.write_all(&e_response).await {
                                eprint!("[Error] # Unable to write to {addr}");
                                break;
                            }

                            attempts += 1;
                            eprint!(
                                "[Error] # Unable to connect player {addr}...Attempts: {attempts}"
                            );
                        }
                    }
                    _ => {
                        let body = b"Invalid header";
                        let response = Protocol::create_packet(HeaderTypes::Err, body);
                        if let Err(_) = c_stream.write_all(&response).await {
                            eprint!("[Error] # Unable to write to {addr}");
                            break;
                        };
                    }
                }
            } else {
                let body = b"Invalid header";
                let response = Protocol::create_packet(HeaderTypes::Err, body);
                if let Err(_) = c_stream.write_all(&response).await {
                    eprint!("[Error] # Unable to write to {addr}");
                    break;
                };
            }
        }

        println!("[Close] # Closing connection with {addr}");
        if let Some(player_id) = player_id {
            server.remove_player(&player_id).await;
        }
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                println!("[Incoming]# {addr}");
                let server_clone = Arc::clone(&self);
                tokio::spawn(ServerInstance::handle_client(server_clone, c_stream, addr));
            }
        }
    }

    async fn add_player(&self, player: PlayerState) {
        let mut players = self.player_state.write().await;
        players.insert(player.id.clone(), player);
    }

    async fn remove_player(&self, id: &str) {
        let mut players = self.player_state.write().await;
        players.remove(id);
    }
}
