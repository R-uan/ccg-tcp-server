use std::sync::{Arc, Mutex};

use tokio::net::TcpStream;

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

pub struct PlayerState {
    pub id: String,
    pub position: Vec2,
    pub nickname: String,
    pub stream: Arc<Mutex<TcpStream>>,
}

impl PlayerState {
    pub fn forge_connection(protocol: &[u8]) -> Self {
        todo!()
    }
}
