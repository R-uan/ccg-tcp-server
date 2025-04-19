use std::{io::Error, sync::Arc};

use tcp::server::ServerInstance;

mod game;
mod models;
mod tcp;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port = 8000;
    if let Ok(server) = ServerInstance::create_instance(port).await {
        let server_arc = Arc::new(server);
        server_arc.run().await;
    }
    Ok(())
}
