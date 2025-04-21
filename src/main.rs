use std::{io::Error, sync::Arc};

use config::{Config, File};
use models::settings::Settings;
use tcp::server::ServerInstance;
use tokio::sync::OnceCell;

mod game;
mod models;
mod tcp;
mod utils;

static SETTINGS: OnceCell<Settings> = OnceCell::const_new();

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
    if let Ok(server) = ServerInstance::create_instance(port).await {
        let server_arc = Arc::new(server);
        server_arc.run().await;
    }
    Ok(())
}
