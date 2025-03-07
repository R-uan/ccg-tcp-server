use std::{io::Error, net::Ipv4Addr};
use tokio::net::TcpListener;

use crate::game::{game_state::GameState, player_state::PlayerState};

pub struct ServerInstance {
    pub server_port: u16,
    pub socket: TcpListener,
    pub game_state: GameState,
    pub player_state: Vec<PlayerState>,
}

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

impl ServerInstance {
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => Ok(ServerInstance {
                server_port: port,
                socket: tcp_stream,
                game_state: GameState::new_game(),
                player_state: vec![],
            }),
            Err(error) => Err(error),
        };
    }
}
