use super::client::{Client, TemporaryClient};
use crate::game::lua_context::LuaContext;
use crate::models::client_requests::PlayCardRequest;
use crate::models::deck::Card;
use crate::tcp::header::{Header, HeaderType};
use crate::tcp::server::ServerInstance;
use crate::utils::errors::{GameLogicError, NetworkError, PlayerConnectionError};
use crate::{
    game::player::Player,
    utils::{checksum::CheckSum, errors::ProtocolError, logger::Logger},
};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time;
use crate::tcp::packet::Packet;

pub struct Protocol {
    pub server: Arc<ServerInstance>,
}

impl Protocol {
    pub fn new(server: Arc<ServerInstance>) -> Self {
        Protocol { server }
    }

    pub async fn handle_incoming(&self, client: Arc<Client>, buffer: &[u8]) {
        let addr = client.addr.read().await;
        if let Ok(packet) = Packet::parse(&buffer) {
            Logger::debug(&format!(
                "[PROTOCOL] Received packet: {{ type: {}, size: {} }}",
                packet.header.header_type.to_string(),
                packet.header.payload_length
            ));
            if !CheckSum::check(&packet.header.checksum, &packet.payload) {
                Logger::error("[PROTOCOL] Invalid checksum value");
                drop(addr);
                let packet = Packet::new(HeaderType::InvalidChecksum, b"");
                self.send_or_disconnect(client, &packet).await;
                return;
            } else {
                Logger::error(&format!("[PROTOCOL] Failed to parse packet from `{addr}`"));
            }
            drop(addr);
            self.handle_packet(client, &packet).await
        } else {
            Logger::info("[PROTOCOL] Unable to parse packet");
        }
    }

    /// Attempts to send a packet to the client, retrying up to 3 times on failure.
    ///
    /// - Serializes the packet and writes it to the client's stream.
    /// - Waits 500ms between retries if sending fails.
    /// - Returns `Err(PackageWriteError)` after 3 failed attempts.
    ///
    /// Logs all outcomes
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
                Logger::error(&format!(
                    "[PROTOCOL] Failed to send packet to `{addr}`. Retrying... [{}/3]",
                    tries + 1
                ));
                tokio::time::sleep(Duration::from_millis(500)).await;
                tries += 1;
                continue;
            }

            Logger::debug(&format!(
                "[PROTOCOL] Sent packet {{ type: {}, size: {} }} to `{addr}`",
                packet.header.header_type.to_string(),
                packet_data.len(),
            ));
            return Ok(());
        }

        self.disconnect(client).await;
        Err(NetworkError::PackageWriteError("unknown error".to_string()))
    }

    
    async fn disconnect(&self, client: Arc<Client>) {
        let addr = client.addr.read().await;
        Logger::info(&format!("[PROTOCOL] Client `{addr}` disconnected"));
        let mut connected_guard = client.connected.write().await;
        *connected_guard = false;
    }

    /// Sends a packet to the client, disconnecting if the send fails.
    ///
    /// Useful for simplifying repeated send-and-disconnect patterns.
    /// Prevents duplicated error handling logic throughout packet handling.
    async fn send_or_disconnect(&self, client: Arc<Client>, packet: &Packet) {
        let client_clone = Arc::clone(&client);
        if self.send_packet(client, packet).await.is_err() {
            self.disconnect(client_clone).await;
        }
    }

    async fn send_and_disconnect(&self, client: Arc<Client>, packet: &Packet) {
        let client_clone = Arc::clone(&client);
        let _ = self.send_packet(client, packet).await;
        self.disconnect(client_clone).await;
    }

    async fn handle_packet(&self, client: Arc<Client>, packet: &Packet) {
        let message_type = &packet.header.header_type;
        match message_type {
            HeaderType::Disconnect => self.handle_disconnect(client).await,
            HeaderType::PlayCard => {
                if let Ok(request) = serde_cbor::from_slice::<PlayCardRequest>(&packet.payload) {
                    let play_card = self.handle_play_card(client, &request).await;
                } else {
                    let invalid_request = Packet::new(
                        HeaderType::InvalidPacketPayload,
                        b"Could not parse play card request.",
                    );
                    let _ = self.send_packet(client.clone(), &invalid_request).await;
                }
            }
            _ => {
                Logger::warn("[PROTOCOL] Invalid header");
                let packet = Packet::new(HeaderType::InvalidHeader, b"");
                self.send_or_disconnect(client, &packet).await;
            }
        }
    }

    pub async fn handle_connect(
        self: Arc<Self>,
        temp_client: Arc<TemporaryClient>,
        packet: &Packet,
    ) -> Result<(), PlayerConnectionError> {
        let player = Player::new_connection(&packet.payload).await?;
        Logger::info(&format!(
            "[PROTOCOL] Client `{}` successfully authenticated as `{}`",
            &temp_client.addr, &player.username
        ));
        match Arc::try_unwrap(temp_client) {
            Ok(temp) => {
                let player_id_clone = player.id.clone();
                let addr = temp.addr;
                let (read, write) = temp.stream.into_split();
                let client = Arc::new(Client::new(read, write, addr, player, Arc::clone(&self)));
                let mut players_guard = self.server.players.write().await;
                players_guard.insert(player_id_clone, Arc::clone(&client));
                tokio::spawn(async move {
                    client.connect().await;
                });

                Ok(())
            }
            Err(_) => Err(PlayerConnectionError::InternalError(
                "Failed to unwrap temporary client".to_string(),
            )),
        }
    }

    pub async fn handle_reconnect(
        self: Arc<Self>,
        temp_client: Arc<TemporaryClient>,
        packet: &Packet,
    ) -> Result<(), PlayerConnectionError> {
        Logger::info(&format!(
            "[PROTOCOL] Reconnection request from `{}`",
            &temp_client.addr
        ));
        let authenticated_player = Player::reconnection(&packet.payload).await?;
        Logger::info(&format!(
            "[PROTOCOL] Client `{}` has been authenticated as player `{}`.",
            &temp_client.addr, &authenticated_player.username
        ));
        let players_map = self.server.players.read().await;
        
        if let Some(client) = players_map.get(&authenticated_player.player_id) {
            match Arc::try_unwrap(temp_client) {
                Ok(temp) => {
                    Logger::info(&format!(
                        "[PROTOCOL] Attempting to reconnect player `{}`",
                        &client.player.read().await.username
                    ));

                    let client_clone = Arc::clone(&client);
                    client_clone.reconnect(temp).await;
                    return Ok(());
                }
                Err(_) => Err(PlayerConnectionError::InternalError(
                    "Failed to unwrap temporary client".to_string(),
                )),
            }
        } else {
            Logger::error(&format!(
                "[PROTOCOL] Player `{}` not connected to this match",
                &temp_client.addr
            ));
            Err(PlayerConnectionError::InternalError(
                "Player not found in this match".to_string(),
            ))
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
    /// - Retrieves the full card data (fetching from external source if necessary).
    /// - Executes the card’s `on_play` triggers via the Lua scripting engine.
    ///
    /// # Arguments
    /// * `client` - The client attempting to play the card.
    /// * `request` - The play card request containing the player and card ID.
    ///
    /// # Returns
    /// * `Ok(())` if the action is successful.
    /// * `Err(GameLogicError)` if any validation or execution step fails.
    async fn handle_play_card(
        &self,
        client: Arc<Client>,
        request: &PlayCardRequest,
    ) -> Result<(), GameLogicError> {
        let game_state = self.server.game_state.read().await;
        // Try to fetch the PrivatePlayerView for the given player ID. Return an error if not found.
        let player_view = game_state.players.get(&request.player_id).ok_or_else(|| {
            Logger::error(&format!("Player `{}` was not found", &request.player_id));
            return GameLogicError::PlayerNotFound;
        })?;

        let private_player_view_clone = Arc::clone(player_view);
        let private_player_view_guard = private_player_view_clone.read().await;

        // Clone and lock the Client player object to compare identity and access full player data.
        let player_clone = Arc::clone(&client.player);
        let player_guard = player_clone.read().await;

        // Ensure that the client attempting the action matches the player in the request.
        if &player_guard.id != &private_player_view_guard.id {
            Logger::warn(&format!(
                "Client's player ID ({}) does not match request's ({})",
                &player_guard.id, &request.player_id
            ));
            return Err(GameLogicError::PlayerIdDoesNotMatch);
        }

        //Confirm it is currently this player's turn.
        if &private_player_view_guard.id != &request.player_id {
            Logger::warn(&format!(
                "It's not player's turn: {}",
                &player_guard.username
            ));
            return Err(GameLogicError::NotPlayerTurn);
        }

        // Verifies if the card played is actually in the player's hand. This does not account for
        // out of hand plays from special interactions as they do not exist yet.
        let player_hand = private_player_view_guard.current_hand.iter();
        let card_view = player_hand
            .flatten()
            .find(|c| c.id == request.card_id)
            .ok_or_else(|| {
                Logger::warn(&format!(
                    "Card player is not in player's ({}) hand",
                    &player_guard.username
                ));
                return GameLogicError::CardPlayedIsNotInHand;
            })?;

        // Verify that the requested card is in the player's current hand.
        // Retrieve the full card details from game_cards. If not present, fetch from external storage and add it to the shared card list.
        let game_cards_lock = game_state.game_cards.read().await;
        let full_card = match game_cards_lock.get(&card_view.id) {
            Some(card) => card,
            None => {
                let card = Card::request_card(&card_view.id)
                    .await
                    .map_err(|_| GameLogicError::UnableToGetCardDetails)?;
                game_state.add_card(card).await;
                game_cards_lock.get(&card_view.id).ok_or_else(|| {
                    return GameLogicError::UnableToGetCardDetails;
                })?
            }
        };

        // Iterate over the card’s on_play triggers, creating a Lua execution context for each.
        for action in &full_card.on_play {
            let lua_context = LuaContext::new(
                Arc::clone(&self.server.game_state),
                card_view,
                None,
                "on_play".to_string(),
                action.to_string(),
            )
            .await;

            // Execute each script action using the ScriptManager and apply the resulting game actions to the state.
            let script_manager_clone = Arc::clone(&self.server.scripts);
            let script_manager_guard = script_manager_clone.read().await;
            let game_actions = script_manager_guard
                .call_function_ctx(action, lua_context)
                .await?;

            game_state.apply_actions(game_actions).await;
        }

        Ok(())
    }

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
        Logger::debug(&format!(
            "[PROTOCOL] Sent latest missed packets to {}",
            &client.addr.read().await
        ));
    }

    pub async fn cycle_game_state(&self) {
        let game_state = Arc::clone(&self.server.game_state);
        let game_state_guard = game_state.read().await;

        let mut interval = time::interval(Duration::from_millis(1000));
        while *game_state_guard.ongoing.read().await {
            let game_state_bytes = game_state_guard.wrap_game_state();
            let transmitter_clone = Arc::clone(&self.server.transmitter);
            let transmitter_guard = transmitter_clone.lock().await;
            let game_state_packet = Packet::new(HeaderType::GameState, &game_state_bytes);
            let _ = transmitter_guard.send(game_state_packet);
            interval.tick().await;
        }
    }
}