use tokio::net::TcpStream;

pub struct PlayerState {
    id: String,
    nickname: String,
    stream: TcpStream,
}

impl PlayerState {
    pub fn forge_connection(protocol: String) {}
}
