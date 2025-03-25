use std::{
    collections::HashMap,
    io::Error,
    net::{IpAddr, SocketAddr},
    sync::{Arc, LazyLock},
};

use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        return Self { x, y };
    }
}

type SharedPlayerState = Arc<RwLock<HashMap<String, Player>>>;

pub static PLAYER_STATE: LazyLock<SharedPlayerState> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct Player {
    pub id: String,
    pub position: Vec2,
    pub nickname: String,
    pub ip_addr: IpAddr,
}

impl Player {
    /**
    # Arguments
    * `body`: An array of u8 bytes that contains the valid protocol body for a player connection request.

    # Returns
    * Player with the data extracted from the protocol.
    */
    pub fn new(body: &[u8], ip_addr: &SocketAddr) -> Result<Self, Error> {
        let fields: Vec<&str> = std::str::from_utf8(body)
            .expect("Couldn't read bytes")
            .split("\n")
            .collect();

        if fields.len() < 2 {
            return Err(Error::last_os_error());
        }

        return Ok(Self {
            id: fields[0].to_owned(),
            nickname: fields[1].to_owned(),
            position: Vec2::new(0.0, 0.0),
            ip_addr: ip_addr.to_owned().ip(),
        });
    }

    pub async fn add_player(player: Player) {
        let mut players = PLAYER_STATE.write().await;
        players.insert(player.id.clone(), player);
    }

    pub async fn remove_player(id: &str) {
        let mut players = PLAYER_STATE.write().await;
        players.remove(id);
    }

    pub fn update_position(&mut self, payload: &[u8]) {
        if payload.len() == 4 {
            let x = u16::from_be_bytes([payload[0], payload[1]]);
            let y = u16::from_be_bytes([payload[2], payload[2]]);

            self.position.x = x as f32;
            self.position.y = y as f32;
        }
    }
}
