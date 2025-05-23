use super::client::{Client, TemporaryClient};
use crate::tcp::server::ServerInstance;
use crate::utils::errors::{NetworkError, PlayerConnectionError};
use crate::{
    game::{lua_context::LuaContext, player::Player},
    utils::{checksum::CheckSum, errors::ProtocolError, logger::Logger},
};
use std::net::SocketAddr;
use std::ops::Deref;
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
            Logger::info(&format!("{addr}: packet successfully parsed."));
            if !CheckSum::check(&packet.header.checksum, &packet.payload) {
                Logger::error(&format!("{addr}: invalid checksum."));
                drop(addr);
                let packet = Packet::new(MessageType::InvalidChecksum, b"");
                self.send_or_disconnect(client, &packet).await;
                return;
            }
            drop(addr);
            self.handle_packet(client, &packet).await
        } else {
            Logger::info(&format!("{addr}: packet couldn't be parsed."));
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
        write_half: Arc<RwLock<OwnedWriteHalf>>,
        addr: SocketAddr,
        packet: &Packet,
    ) -> Result<(), NetworkError> {
        let mut tries = 0;
        while tries < 3 {
            let packet_data = packet.wrap_packet();
            let mut stream_guard = write_half.write().await;
            if stream_guard.write_all(&packet_data).await.is_err() {
                Logger::error(&format!(
                    "{}: failed to send packet. Retrying... [{}]",
                    addr, tries
                ));
                tokio::time::sleep(Duration::from_millis(500)).await;
                tries += 1;
                continue;
            }

            Logger::info(&format!("{}: {} bytes sent", addr, packet_data.len()));
            return Ok(());
        }
        return Err(NetworkError::PackageWriteError("unknown error".to_string()));
    }

    /// Sends a packet to the client, disconnecting if the send fails.
    ///
    /// Useful for simplifying repeated send-and-disconnect patterns.
    /// Prevents duplicated error handling logic throughout packet handling.
    async fn send_or_disconnect(&self, client: Arc<Client>, packet: &Packet) {
        let addr = client.addr.read().await.clone();
        if self
            .send_packet(client.write_stream.clone(), addr, packet)
            .await
            .is_err()
        {
            self.disconnect(client).await;
        }
    }

    async fn handle_packet(&self, client: Arc<Client>, packet: &Packet) {
        let message_type = &packet.header.header_type;
        match message_type {
            MessageType::Disconnect => self.handle_disconnect(client).await,
            MessageType::PlayCard => self.handle_play_card(client, packet).await,
            _ => {
                {
                    let addr = client.addr.read().await;
                    Logger::warn(&format!("{addr}: invalid header"));
                }
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
        let player = Player::new(&packet.payload).await?;
        Logger::info(&format!(
            "{}: Player successfully authenticated [{}].",
            &temp_client.addr, &player.id
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
        Logger::info(&format!("{}: requesting reconnection.", &temp_client.addr));
        let player = Player::new(&packet.payload).await?;
        Logger::info(&format!(
            "{}: has been authenticated as player [{}].",
            &temp_client.addr, player.id
        ));
        let players_map = self.server.players.read().await;
        if let Some(client) = players_map.get(&player.id) {
            return match Arc::try_unwrap(temp_client) {
                Ok(temp) => {
                    Logger::info(&format!(
                        "{}: player has been found in match. Attempting reconnection.",
                        &temp.addr
                    ));

                    let client_clone = Arc::clone(client);
                    let (read, write) = temp.stream.into_split();

                    let mut write_stream = client_clone.write_stream.write().await;
                    let mut read_stream = client.read_stream.write().await;
                    let mut addr = client.addr.write().await;

                    *write_stream = write;
                    *read_stream = read;
                    *addr = temp.addr;

                    return Ok(());
                }
                Err(_) => Err(PlayerConnectionError::InternalError(
                    "Failed to unwrap temporary client".to_string(),
                )),
            };
        } else {
            return Err(PlayerConnectionError::InternalError("player not found".to_string()));
        }
    }

    /// Gracefully disconnects the client from the server.
    ///
    /// - Logs the disconnection.
    /// - Removes the client from the global `CLIENTS` map.
    /// - Sets its `connected` flag to `false`.
    async fn disconnect(&self, client: Arc<Client>) {
        let addr = client.addr.read().await;
        Logger::info(&format!("{addr}: disconnected"));
        let mut connected_guard = client.connected.write().await;
        *connected_guard = false;
    }

    async fn handle_play_card(&self, client: Arc<Client>, packet: &Packet) {}

    async fn handle_disconnect(&self, client: Arc<Client>) {
        {
            let addr = client.addr.read().await;
            Logger::info(&format!("{addr}: client disconnecting"));
        }
        let packet = Packet::new(MessageType::Disconnect, b"");
        self.send_or_disconnect(client, &packet).await;
    }

    pub async fn cycle_game_state(&self) {
        let game_state = Arc::clone(&self.server.game_state);
        let game_state_guard = game_state.read().await;

        let mut interval = time::interval(std::time::Duration::from_millis(1000));
        while *game_state_guard.ongoing.read().await {
            let game_state_bytes = game_state_guard.wrap_game_state();
            let transmitter_clone = Arc::clone(&self.server.transmitter);
            let transmitter_guard = transmitter_clone.lock().await;
            let game_state_packet = Packet::new(MessageType::GameState, &game_state_bytes);
            Logger::info("sending game state");
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
            Logger::error(&"Protocol size too smol".to_string());
            return Err(ProtocolError::InvalidPacketError("Too small".to_string()));
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

#[cfg(test)]
mod protocol_header_tests {
    use super::*;

    #[test]
    fn test_protocol_header_creation() {
        let payload = &[0x10, 0x20, 0x30];
        let header = ProtocolHeader::new(MessageType::Ping, payload);

        assert_eq!(header.header_type, MessageType::Ping);
        assert_eq!(header.payload_length, 3);
        assert_eq!(header.checksum, CheckSum::new(payload) as i16);
    }

    #[test]
    fn test_protocol_header_wrap_header() {
        let payload = &[0xAA, 0xBB];
        let header = ProtocolHeader::new(MessageType::Ping, payload);
        let bytes = header.wrap_header();

        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], MessageType::Ping as u8);
        assert_eq!(bytes[1], 0x00); // high byte of payload length
        assert_eq!(bytes[2], 0x02); // low byte of payload length

        let expected_checksum = CheckSum::new(payload);
        assert_eq!(bytes[3], ((expected_checksum >> 8) & 0xFF) as u8);
        assert_eq!(bytes[4], (expected_checksum & 0xFF) as u8);

        assert_eq!(bytes[5], 0x0A);
    }

    #[test]
    fn test_protocol_header_from_bytes_valid() {
        let payload = &[0x01, 0x02];
        let header = ProtocolHeader::new(MessageType::Ping, payload);
        let bytes = header.wrap_header();

        let parsed = ProtocolHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header_type, MessageType::Ping);
        assert_eq!(parsed.payload_length, 2);
        assert_eq!(parsed.checksum, CheckSum::new(payload) as i16);
    }

    #[test]
    fn test_protocol_header_from_bytes_too_short() {
        let bytes = &[0x01, 0x00];
        let result = ProtocolHeader::from_bytes(bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_header_from_bytes_invalid_type() {
        let payload_length = 1u16.to_be_bytes();
        let checksum = 0x1234u16.to_be_bytes();
        let bytes = [
            0xFF, // invalid message type
            payload_length[0],
            payload_length[1],
            checksum[0],
            checksum[1],
        ];
        let result = ProtocolHeader::from_bytes(&bytes);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[test]
    fn test_packet_new_and_fields() {
        let payload = &[0xDE, 0xAD, 0xBE, 0xEF];
        let packet = Packet::new(MessageType::Ping, payload);

        assert_eq!(packet.header.header_type, MessageType::Ping);
        assert_eq!(packet.header.payload_length, 4);
        assert_eq!(packet.header.checksum, CheckSum::new(payload) as i16);
        assert_eq!(&*packet.payload, payload);
    }

    #[test]
    fn test_packet_wrap_packet() {
        let payload = &[0x42, 0x24];
        let packet = Packet::new(MessageType::Ping, payload);
        let raw = packet.wrap_packet();

        // Should be 6 bytes for header + 2 bytes for payload
        assert_eq!(raw.len(), 8);

        // Delimiter check
        assert_eq!(raw[5], 0x0A);
        // Payload content
        assert_eq!(&raw[6..], payload);
    }

    #[test]
    fn test_packet_parse_valid() {
        let payload = &[0x01, 0x02, 0x03];
        let original = Packet::new(MessageType::Ping, payload);
        let raw = original.wrap_packet();

        let parsed = Packet::parse(&raw).unwrap();
        assert_eq!(
            parsed.header.header_type,
            MessageType::Ping,
            "HeaderType does not match"
        );
        assert_eq!(
            parsed.header.payload_length, 3,
            "Payload length does not match"
        );
        assert_eq!(
            parsed.header.checksum,
            CheckSum::new(payload) as i16,
            "Checksum does not match"
        );
        assert_eq!(&*parsed.payload, payload, "Payload does not match");
    }

    #[test]
    fn test_packet_parse_invalid_header() {
        // Too short to contain full header
        let raw = &[0x01, 0x00, 0x01, 0x12];
        let result = Packet::parse(raw);
        assert!(result.is_err());
    }
}
