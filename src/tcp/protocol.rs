use super::client::{Client, TemporaryClient};
use crate::game::lua_context::LuaContext;
use crate::models::game_action::GameAction;
use crate::tcp::server::ServerInstance;
use crate::utils::errors::{GameLogicError, NetworkError, PlayerConnectionError};
use crate::{
    game::player::Player,
    utils::{checksum::CheckSum, errors::ProtocolError, logger::Logger},
};
use mlua::LuaSerdeExt;
use std::collections::VecDeque;
use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::RwLock;
use tokio::time;

/// Represents the type of message in a protocol packet.
///
/// Each variant maps to a specific `u8` value used during transmission.
///
/// # Variants
///
/// - `Disconnect` - Client is disconnecting.
/// - `Connect` - Client is initiating a connection.
/// - `GameState` - Server is sending current game state.
///
/// ### Errors (0xFB–0xFF):
/// - `AlreadyConnected` - Client is already connected.
/// - `InvalidPlayerData` - Malformed or missing player data.
/// - `InvalidChecksum` - Payload failed checksum validation.
/// - `InvalidHeader` - Malformed or unrecognized header.
/// - `ERROR` - Generic error.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Disconnect = 0x00,
    Connect = 0x01,
    Ping = 0x02,
    Reconnect = 0x03,

    GameState = 0x10,

    PlayCard = 0x11,
    AttackPlayer = 0x12,

    InvalidHeader = 0xFA,
    AlreadyConnected = 0xFB,
    InvalidPlayerData = 0xFC,
    InvalidChecksum = 0xFD,
    FailedToConnectPlayer = 0xF0,
    ERROR = 0xFE,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            MessageType::Disconnect => String::from("DISCONNECT"),
            MessageType::Connect => String::from("CONNECT"),
            MessageType::Reconnect => String::from("RECONNECT"),
            MessageType::Ping => String::from("PING"),

            MessageType::PlayCard => String::from("PLAY_CARD"),
            MessageType::AttackPlayer => String::from("ATTACK_PLAYER"),

            MessageType::InvalidHeader => String::from("INVALID_HEADER"),
            MessageType::AlreadyConnected => String::from("ALREADY_CONNECTED"),
            MessageType::InvalidPlayerData => String::from("INVALID_PLAYER_DATA"),
            MessageType::InvalidChecksum => String::from("INVALID_CHECKSUM"),
            MessageType::FailedToConnectPlayer => String::from("FAILED_TO_CONNECT_PLAYER"),
            MessageType::ERROR => String::from("ERROR"),

            MessageType::GameState => String::from("GAME_STATE"),
        };
        return write!(f, "{}", str);
    }
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    /// Attempts to convert a `u8` into a `MessageType`.
    ///
    /// Returns `Ok(MessageType)` if the byte matches a known variant.
    /// Returns `Err(())` if the byte does not correspond to any defined message type.
    ///
    /// Useful for deserializing incoming packets.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(MessageType::Disconnect),
            0x01 => Ok(MessageType::Connect),
            0x02 => Ok(MessageType::Ping),
            0x03 => Ok(MessageType::Reconnect),

            0x10 => Ok(MessageType::GameState),
            0x11 => Ok(MessageType::PlayCard),
            0x12 => Ok(MessageType::AttackPlayer),

            0xFE => Ok(MessageType::ERROR),
            _ => Err(()),
        }
    }
}

pub struct Protocol {
    pub server: Arc<ServerInstance>,
}

impl Protocol {
    pub fn new(server: Arc<ServerInstance>) -> Self {
        return Protocol { server };
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
                let packet = Packet::new(MessageType::InvalidChecksum, b"");
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
        return Err(NetworkError::PackageWriteError("unknown error".to_string()));
    }

    /// Gracefully disconnects the client from the server.
    ///
    /// - Logs the disconnection.
    /// - Removes the client from the global `CLIENTS` map.
    /// - Sets its `connected` flag to `false`.
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
            MessageType::Disconnect => self.handle_disconnect(client).await,
            MessageType::PlayCard => self
                .handle_play_card(client, packet)
                .await
                .unwrap_or_default(),
            _ => {
                Logger::warn("[PROTOCOL] Invalid header");
                let packet = Packet::new(MessageType::InvalidHeader, b"");
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
        return match Arc::try_unwrap(temp_client) {
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

                return Ok(());
            }
            Err(_) => Err(PlayerConnectionError::InternalError(
                "Failed to unwrap temporary client".to_string(),
            )),
        };
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
        return if let Some(client) = players_map.get(&authenticated_player.player_id) {
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
        };
    }

    async fn handle_disconnect(&self, client: Arc<Client>) {
        let packet = Packet::new(MessageType::Disconnect, b"");
        self.send_and_disconnect(client, &packet).await;
    }

    async fn handle_play_card(
        &self,
        client: Arc<Client>,
        packet: &Packet,
    ) -> Result<(), GameLogicError> {
        let card_id = String::from_utf8_lossy(&packet.payload);
        let game_state = self.server.game_state.read().await;
        if let Some(player_view) = game_state.players.get(&game_state.curr_turn) {
            let player_clone = Arc::clone(player_view);
            let player_guard = player_clone.read().await;
            let hand = player_guard
                .current_hand
                .iter()
                .flatten()
                .find(|c| c.id == card_id);
            if let Some(card_view) = hand {
                let player = Arc::clone(&client.player);
                if player
                    .read()
                    .await
                    .current_deck
                    .cards
                    .iter()
                    .find(|c| c.id == card_view.id)
                    .is_none()
                {
                    return Err(GameLogicError::CardPlayedIsNotInHand);
                }

                let game_cards_lock = game_state.game_cards.read().await;
                if let Some(full_card) = game_cards_lock.iter().find(|c| c.id == card_view.id) {
                    for action in &full_card.on_play {
                        let lua_context = LuaContext::new(
                            Arc::clone(&self.server.game_state),
                            card_view,
                            None,
                            "on_play".to_string(),
                            action.to_string(),
                        )
                        .await;

                        let script_manager_clone = Arc::clone(&self.server.scripts);
                        let script_manager_guard = script_manager_clone.read().await;
                        let game_actions = script_manager_guard
                            .call_function(action, lua_context)
                            .await;
                    }
                } else {
                    // Should fetch it on the spot as the card is already confirmed to be in the deck
                    todo!();
                }
            }
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
            let game_state_packet = Packet::new(MessageType::GameState, &game_state_bytes);
            let _ = transmitter_guard.send(game_state_packet);
            interval.tick().await;
        }
    }
}

/// Represents a fixed-size protocol header for game packet transmission.
///
/// Contains the message type, payload length, and a checksum for validation.
/// Serialized as 6 bytes total when sent over the network.
#[derive(Clone)]
pub struct ProtocolHeader {
    pub checksum: i16,
    pub payload_length: i16,
    pub header_type: MessageType,
}

impl ProtocolHeader {
    /// Creates a new `ProtocolHeader` from the given message type and payload.
    ///
    /// Calculates the checksum and payload length automatically.
    pub fn new(header_type: MessageType, payload: &[u8]) -> Self {
        return Self {
            checksum: CheckSum::new(payload) as i16,
            payload_length: payload.len() as i16,
            header_type,
        };
    }

    /// Serializes the header into a fixed-size byte array.
    ///
    /// Format: [type, payload_len (2 bytes), checksum (2 bytes), 0x0A].
    pub fn wrap_header(&self) -> Box<[u8]> {
        let checksum: u16 = self.checksum as u16;
        let payload_length: u16 = self.payload_length as u16;
        let header_type: u8 = self.header_type.to_owned() as u8;

        return Box::new([
            header_type,
            ((payload_length >> 8) & 0xFF) as u8,
            (payload_length & 0xFF) as u8,
            ((checksum >> 8) & 0xFF) as u8,
            (checksum & 0xFF) as u8,
            0x0A,
        ]);
    }

    /// Parses a `ProtocolHeader` from a byte slice.
    ///
    /// Returns an error if the slice is too short or has an invalid type.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() != 6 || bytes[5] != 0x0A {
            return Err(ProtocolError::InvalidHeaderError(format!(
                "Format invalid: {:?}",
                bytes
            )));
        }

        return match MessageType::try_from(bytes[0]) {
            Err(_) => Err(ProtocolError::InvalidHeaderError(
                "Invalid message type.".to_string(),
            )),
            Ok(header_type) => {
                let checksum: i16 = u16::from_be_bytes([bytes[3], bytes[4]]) as i16;
                let payload_length: i16 = u16::from_be_bytes([bytes[1], bytes[2]]) as i16;

                Ok(Self {
                    header_type,
                    payload_length,
                    checksum,
                })
            }
        };
    }
}

/// Represents a complete network packet with a protocol header and payload.
///
/// Handles serialization and parsing for message transmission.
#[derive(Clone)]
pub struct Packet {
    pub header: ProtocolHeader,
    pub payload: Box<[u8]>,
}

impl Packet {
    /// Parses a raw byte slice into a `Packet`.
    ///
    /// Expects a 5-byte header followed by the payload (skips byte 5: delimiter).
    /// Returns an error if the header is invalid.
    pub fn parse(protocol: &[u8]) -> Result<Self, ProtocolError> {
        if protocol.len() < 6 {
            Logger::error("[PROTOCOL] Not enough bytes for a valid packet");
            return Err(ProtocolError::InvalidPacketError(
                "Not enough bytes for a valid packet".to_string(),
            ));
        }

        let header = ProtocolHeader::from_bytes(&protocol[..6])?;
        let payload = protocol[6..].to_owned().into_boxed_slice();
        return Ok(Self { header, payload });
    }

    /// Creates a new `Packet` from a message type and payload.
    ///
    /// Automatically constructs the header.
    pub fn new(header_type: MessageType, payload: &[u8]) -> Self {
        let header = ProtocolHeader::new(header_type, payload);
        let payload = payload.to_vec().into_boxed_slice();
        return Self { header, payload };
    }

    /// Serializes the packet into a byte slice.
    ///
    /// Concatenates the header and payload into a single buffer.
    pub fn wrap_packet(&self) -> Box<[u8]> {
        let header = self.header.wrap_header();
        let mut packet = Vec::with_capacity(header.len() + self.payload.len());

        packet.extend_from_slice(&header);
        packet.extend_from_slice(&self.payload);

        packet.into_boxed_slice()
    }
}
