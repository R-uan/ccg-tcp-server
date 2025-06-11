use config::{Config, File};
use models::settings::Settings;
use std::{io::Error, sync::Arc};
use std::sync::LazyLock;
use tcp::server::ServerInstance;
use tokio::sync::OnceCell;
use crate::tcp::server::UninitializedServer;

mod game;
mod models;
mod tcp;
mod utils;

static SETTINGS: OnceCell<Settings> = OnceCell::const_new();
static SERVER_INSTANCE: OnceCell<ServerInstance> = OnceCell::const_new();

#[tokio::main]
async fn main() -> Result<(), Error> {
    SETTINGS
        .set(
            Config::builder()
                .add_source(File::with_name("config"))
                .build()
                .unwrap()
                .try_deserialize::<Settings>()
                .unwrap(),
        )
        .unwrap();

    let port = 8000;
    
    if let Ok(uninitialized) = UninitializedServer::create_instance(port).await {
        let server_arc = Arc::new(uninitialized);
        if let Ok(initialized_server) = Arc::clone(&server_arc).await_for_initialization().await {
            let initialized_clone = Arc::new(initialized_server);
            initialized_clone.listen().await;
        }
    }

    Ok(())
}
