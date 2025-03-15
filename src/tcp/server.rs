use std::{
    io::Error,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    game::{
        game_state::GameState,
        player_state::{Player, SHARED_PLAYER_STATE},
    },
    tcp::protocol::{CheckSum, Protocol, ProtocolHeader},
};

use super::protocol::ProtocolOperations;

pub struct ServerInstance {
    pub socket: TcpListener,
    pub game_state: GameState,
}

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

impl ServerInstance {
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
            if let Ok(header) = ProtocolHeader::from_bytes(&buffer[..5]) {
                let protocol_body = &buffer[6..bytes_read - 1];

                if CheckSum::check(&header.checksum, protocol_body) == false {
                    eprint!("[Error] # Checksum check failed.");

                    let e_body = b"Checksum failed";
                    let e_response = Protocol::create_packet(ProtocolOperations::Err, e_body);

                    if let Err(_) = c_stream.write_all(&e_response).await {
                        eprint!("[Error] # Unable to write to {addr}");
                        break;
                    }
                };

                match header.operation {
                    ProtocolOperations::Connect => {
                        if let Ok(player) = Player::new(protocol_body, &addr) {
                            player_id = Some(player.id.clone());
                            Player::add_player(player).await;

                            let r_body = b"Player sucessfully connected";
                            let response = Protocol::create_packet(
                                ProtocolOperations::PlayerConnected,
                                r_body,
                            );

                            if let Err(_) = c_stream.write_all(&response).await {
                                eprint!("[Error] # Unable to write to {addr}");
                                break;
                            }
                        } else {
                            let r_body = b"Unable to connect player";
                            let e_response =
                                Protocol::create_packet(ProtocolOperations::Err, r_body);

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
                    ProtocolOperations::Close => {
                        c_stream.write_all(b"0x00").await.unwrap_or_default();
                        break;
                    }
                    ProtocolOperations::PlayerMovement => {
                        if let Some(id) = &player_id {
                            if let Some(player) = SHARED_PLAYER_STATE.write().await.get_mut(id) {
                                player.update_position(protocol_body);
                            };
                        }
                    }
                    _ => {
                        let e_body = b"Invalid header";
                        let e_response = Protocol::create_packet(ProtocolOperations::Err, e_body);
                        if let Err(_) = c_stream.write_all(&e_response).await {
                            eprint!("[Error] # Unable to write to {addr}");
                            break;
                        };
                    }
                }
            } else {
                let e_body = b"Invalid header";
                let e_repose = Protocol::create_packet(ProtocolOperations::Err, e_body);
                if let Err(_) = c_stream.write_all(&e_repose).await {
                    eprint!("[Error] # Unable to write to {addr}");
                    break;
                };
            }
        }

        println!("[Close] # Closing connection with {addr}");
        if let Some(player_id) = player_id {
            Player::remove_player(&player_id).await;
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
}
