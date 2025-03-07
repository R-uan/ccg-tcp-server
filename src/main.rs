use std::io::Error;

use tcp::{protocol::HeaderTypes, server::ServerInstance};
use tokio::io::AsyncReadExt;

mod game;
mod tcp;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = 8000;
    if let Ok(server) = ServerInstance::create_instance(port).await {
        loop {
            if let Ok((mut stream, addr)) = server.socket.accept().await {
                let mut buffer = [0; 1024];
                let bytes_read = stream.read(&mut buffer).await.unwrap();

                if bytes_read <= 0 {
                    drop(stream);
                    continue;
                }

                match HeaderTypes::try_from(buffer[0]).unwrap() {
                    HeaderTypes::Connect => {}
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
