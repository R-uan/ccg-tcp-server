use config::{Config, File};
use models::settings::Settings;
use std::{io::Error, sync::Arc};
use tcp::{
    client::CLIENTS,
    protocol::{MessageType, Packet},
    server::ServerInstance,
};
use tokio::{
    sync::{broadcast::Sender, Mutex, OnceCell},
    time,
};
use utils::logger::Logger;

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
    if let Ok(mut server) = ServerInstance::create_instance(port).await {
        tokio::spawn({
            let server_clone = Arc::new(&server);
            let tx = Arc::clone(&server_clone.transmiter);
            async move { ServerInstance::write_state_update(tx).await }
        });

        server.listen().await;
    }

    Ok(())
}
