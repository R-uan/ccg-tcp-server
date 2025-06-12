use super::client::{Client, TemporaryClient};
use crate::game::entity::player::{Player, PlayerView};
use crate::game::game::GameInstance;
use crate::models::client_requests::PlayCardRequest;
use crate::models::exit_code::ExitCode;
use crate::tcp::header::HeaderType;
use crate::tcp::header::HeaderType::PlayCard;
use crate::tcp::packet::Packet;
use crate::tcp::server::ServerInstance;
use crate::utils::errors::{NetworkError, PlayerConnectionError};
use crate::{
    logger,
    utils::{checksum::Checksum, logger::Logger},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast::Sender;
use tokio::sync::{broadcast, Mutex, RwLock};

/// The Protocol struct handles the communication protocol for the server, managing client connections and packet processing.
pub struct Protocol {
    pub game_instance: Arc<GameInstance>,
    pub server_instance: Arc<ServerInstance>,
    pub transmitter: Arc<Mutex<Sender<Packet>>>, // The transmitter for broadcasting packets to clients.
}

impl Protocol {
    pub fn new(server_instance: Arc<ServerInstance>, game_instance: Arc<GameInstance>) -> Self {
        let (tx, _) = broadcast::channel::<Packet>(10);
        Protocol {
            game_instance,
            server_instance,
            transmitter: Arc::new(Mutex::new(tx)),
        }
    }

    /// Handles incoming packets from a client.
    ///
    /// - Parses the packet from the provided buffer.
    /// - Validates the packet's checksum.
    /// - Logs the packet details.
    /// - If the packet is valid, it calls `handle_packet` to process it.
    /// - If the checksum is invalid, it sends an `InvalidChecksum` packet to the client and disconnects.
    ///
    /// # Arguments
    /// * `client` - The client that sent the packet.
    /// * `buffer` - The byte buffer containing the incoming packet data.
    ///
    /// # Returns
    /// * None if the packet is processed successfully.
    /// * Sends an `InvalidChecksum` packet and disconnects the client if the checksum is invalid.
    ///
    /// Log all outcomes, including errors and successful packet processing.
    pub async fn handle_incoming(&self, client: Arc<Client>, buffer: &[u8]) {
        match Packet::parse(&buffer) {
            Err(error) => logger!(ERROR, "{}", error.to_string()),
            Ok(packet) => {
                logger!(
                    DEBUG,
                    "[PROTOCOL] Received packet: {{ type: {}, size: {} }}",
                    packet.header.header_type.to_string(),
                    packet.header.payload_length
                );

                if !Checksum::check(&packet.header.checksum, &packet.payload) {
                    logger!(WARN, "[PROTOCOL] Invalid checksum value");
                    let packet = Packet::new(HeaderType::InvalidChecksum, b"");
                    self.send_or_disconnect(client, &packet).await;
                    return;
                }
                self.handle_packet(client, &packet).await
            }
        }
    }

    /// Sends a packet to the client, retrying up to 3 times if the sending fails.
    ///
    /// If all attempts fail, it disconnects the client and returns an error.
    ///
    /// # Arguments
    /// * `client` - The client to which the packet should be sent.
    /// * `packet` - The packet to send.
    ///
    /// # Returns
    /// * `Ok(())` if the packet was sent successfully.
    /// * `Err(NetworkError)` if the packet could not be sent after 3 attempts.
    pub async fn send_packet(
        &self,
        client: Arc<Client>,
        packet: &Packet,
    ) -> Result<(), NetworkError> {
        let mut tries = 0;
        while tries < 3 {
            let addr = client.addr.read().await;
            let packet_data = packet.wrap_packet();
            let mut stream_guard = client.write_stream.write().await;
            if stream_guard.write_all(&packet_data).await.is_err() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                tries += 1;
                continue;
            }

            logger!(
                DEBUG,
                "[PROTOCOL] Sent packet {{ type: {}, size: {} }} to `{addr}`",
                packet.header.header_type.to_string(),
                packet_data.len()
            );
            return Ok(());
        }

        Err(NetworkError::PackageWriteError("Unknown error".to_string()))
    }

    /// Disconnects a client by setting its connected state to false and logging the disconnection.
    ///
    /// # Arguments
    /// * `client` - The client to disconnect.
    ///
    /// This function updates the client's connection status and logs the disconnection event.
    ///
    /// It does not send any packets to the client; it simply marks the client as disconnected.
    async fn disconnect(&self, client: Arc<Client>) {
        let addr = client.addr.read().await;
        logger!(INFO, "[PROTOCOL] Client `{addr}` disconnected");
        let mut connected_guard = client.connected.write().await;
        *connected_guard = false;
    }

    /// Sends a packet to the client, and if it fails, it attempts to disconnect the client.
    ///
    /// # Arguments
    /// * `client` - The client to which the packet should be sent.
    /// * `packet` - The packet to send.
    async fn send_or_disconnect(&self, client: Arc<Client>, packet: &Packet) {
        let client_clone = Arc::clone(&client);
        if self.send_packet(client, packet).await.is_err() {
            self.disconnect(client_clone).await;
        }
    }

    /// Sends a packet to the client and then disconnects the client independent of the result.
    ///
    /// # Arguments
    /// * `client` - The client to which the packet should be sent.
    /// * `packet` - The packet to send.
    async fn send_and_disconnect(&self, client: Arc<Client>, packet: &Packet) {
        let client_clone = Arc::clone(&client);
        let _ = self.send_packet(client, packet).await;
        self.disconnect(client_clone).await;
    }

    /// Handles a packet received from a client based on its header type.
    async fn handle_packet(&self, client: Arc<Client>, packet: &Packet) {
        let message_type = &packet.header.header_type;
        match message_type {
            HeaderType::Disconnect => self.handle_disconnect(client).await,
            HeaderType::PlayCard => self.handle_play_card(client, &packet).await,
            _ => {
                logger!(WARN, "[PROTOCOL] Invalid header");
                let packet = Packet::new(HeaderType::InvalidHeader, b"");
                self.send_or_disconnect(client, &packet).await;
            }
        }
    }

    /// Handles a new connection request from a temporary client.
    ///
    /// This function authenticates the player based on the provided packet payload.
    /// If the authentication is successful, it creates a new `Client` instance and adds it to the server's player list.
    /// If the temporary client cannot be unwrapped, it returns an error.
    /// # Arguments
    /// * `temp_client` - The temporary client that is attempting to connect.
    /// * `packet` - The packet containing the authentication payload.
    ///
    /// # Returns
    /// * `Ok(())` if the connection is successfully established.
    /// * `Err(PlayerConnectionError)` if there is an error during the connection process.
    pub async fn handle_connect(
        self: Arc<Self>,
        temp_client: Arc<TemporaryClient>,
        packet: &Packet,
    ) -> Result<(), PlayerConnectionError> {
        let player_authentication = Player::new_connection(&packet.payload).await?;
        logger!(
            INFO,
            "[PROTOCOL] Client `{}` has been authenticated as player `{}`.",
            &temp_client.addr,
            &player_authentication.username
        );

        let connected_players = self
            .server_instance
            .game_instance
            .connected_players
            .read()
            .await;

        if let Some(connected_player) = connected_players.get(&player_authentication.player_id) {
            match Arc::try_unwrap(temp_client) {
                Ok(temp) => {
                    let (read, write) = temp.stream.into_split();
                    let client = Arc::new(Client::new(
                        read,
                        write,
                        temp.addr,
                        self.clone(),
                        connected_player.clone(),
                    ));
                    let mut clients_guard = self.server_instance.connected_clients.write().await;
                    clients_guard.insert(player_authentication.player_id, client.clone());

                    tokio::spawn({
                        async move {
                            client.clone().connect().await;
                        }
                    });

                    Ok(())
                }
                Err(_) => Err(PlayerConnectionError::InternalError(
                    "Unable to unwrap temporary client".to_string(),
                )),
            }
        } else {
            Err(PlayerConnectionError::PlayerNotConnected)
        }
    }

    /// Handles a reconnection request from a temporary client.
    ///
    /// This function attempts to authenticate the player based on the provided packet payload.
    /// If the player is found in the server's player list, it attempts to reconnect the player.
    /// If the temporary client cannot be unwrapped, it returns an error.
    /// If the player is not found, it returns an error indicating that the player is not connected to the match.
    ///
    /// # Arguments
    /// * `temp_client` - The temporary client that is attempting to reconnect.
    /// * `packet` - The packet containing the authentication payload.
    ///
    /// # Returns
    /// * `Ok(())` if the reconnection is successfully established.
    /// * `Err(PlayerConnectionError)` if there is an error during the reconnection process.
    pub async fn handle_reconnect(
        self: Arc<Self>,
        temp_client: Arc<TemporaryClient>,
        packet: &Packet,
    ) -> Result<(), PlayerConnectionError> {
        logger!(
            INFO,
            "[PROTOCOL] Reconnection request from `{}`",
            &temp_client.addr
        );

        let authenticated_player = Player::reconnection(&packet.payload).await?;
        logger!(
            INFO,
            "[PROTOCOL] Client `{}` has been authenticated as player `{}`.",
            &temp_client.addr,
            &authenticated_player.username
        );

        let players_map = self.server_instance.connected_clients.read().await;
        if let Some(client) = players_map.get(&authenticated_player.player_id) {
            match Arc::try_unwrap(temp_client) {
                Err(_) => Err(PlayerConnectionError::InternalError(
                    "Unable to unwrap temporary client".to_string(),
                )),

                Ok(temp) => {
                    logger!(
                        INFO,
                        "[PROTOCOL] Attempting to reconnect player `{}`",
                        &client.player.read().await.username
                    );

                    let client_clone = Arc::clone(&client);
                    client_clone.reconnect(temp).await;

                    Ok(())
                }
            }
        } else {
            Err(PlayerConnectionError::PlayerNotConnected)
        }
    }

    async fn handle_disconnect(&self, client: Arc<Client>) {
        let packet = Packet::new(HeaderType::Disconnect, b"");
        self.send_and_disconnect(client, &packet).await;
    }

    /// Handles a play card action from a client during a game turn.
    ///
    /// This function verifies the legitimacy of the card play request by performing several checks:
    /// - Ensures the player exists in the current game state.
    /// - Validates that the requesting client matches the internal player representation.
    /// - Confirms it is the requesting player’s turn.
    /// - Verifies the card is present in the player’s hand.
    /// - Retrieves the full card data (fetching from an external source if necessary).
    /// - Executes the card’s `on_play` triggers via the Lua scripting engine.
    ///
    /// # Arguments
    /// * `client` - The client attempting to play the card.
    /// * `request` - The play card request containing the player and card ID.
    ///
    /// # Returns
    /// * `Ok(())` if the action is successful.
    /// * `Err(GameLogicError)` if any validation or execution step fails.
    async fn handle_play_card(&self, client: Arc<Client>, packet: &Packet) {
        logger!(DEBUG, "Handle play card ended");
        match serde_cbor::from_slice::<PlayCardRequest>(&packet.payload) {
            Ok(request) => {
                if let Err(error) = self
                    .game_instance
                    .clone()
                    .play_card(client.clone(), &request)
                    .await
                {
                    let error_message = error.to_string();
                    logger!(ERROR, "Play Card Request: {}", error_message.clone());
                    let error_packet = Packet::new(HeaderType::PlayCard, error_message.as_bytes());
                    let _ = self.send_packet(client, &error_packet).await;
                } else {
                    logger!(INFO, "Play card request was finished successfully");
                }
            }
            Err(error) => {
                let error_message = error.to_string();
                logger!(
                    ERROR,
                    "[PROTOCOL] Play card request: {}",
                    error_message.clone()
                );
                let error_packet = Packet::new(HeaderType::PlayCard, error_message.as_bytes());
                let _ = self.send_packet(client, &error_packet).await;
            }
        }
    }

    /// Sends any missed packets to the client.
    ///
    /// This function retrieves the missed packets from the client's queue and sends them one by one.
    /// It uses a loop to send each packet, waiting for a short duration between sending to avoid overwhelming the client.
    ///
    /// # Arguments
    /// * `client` - The client to which the missed packets should be sent.
    pub async fn send_missed_packets(&self, client: Arc<Client>) {
        let mut packets_lock = client.missed_packets.write().await;
        loop {
            if let Some(packet) = packets_lock.pop_front() {
                let client_clone = Arc::clone(&client);
                self.send_or_disconnect(client_clone, &packet).await;
                tokio::time::interval(Duration::from_micros(30))
                    .tick()
                    .await;
            } else {
                break;
            }
        }
        logger!(
            INFO,
            "[PROTOCOL] Sent latest missed packets to {}",
            &client.addr.read().await
        )
    }
}
