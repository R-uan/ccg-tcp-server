use config::{Config, File};
use models::settings::Settings;
use std::io::Error;
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
    if let Ok(mut server) = ServerInstance::create_instance(port).await {
        server.listen().await;
    }

    Ok(())
}
