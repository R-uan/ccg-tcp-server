use std::fmt::Display;
use crate::utils::checksum::CheckSum;
use crate::utils::errors::ProtocolError;

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
pub enum HeaderType {
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
    InvalidPacketPayload = 0xF1,
    ERROR = 0xFE,
}

impl Display for HeaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            HeaderType::Disconnect => String::from("DISCONNECT"),
            HeaderType::Connect => String::from("CONNECT"),
            HeaderType::Reconnect => String::from("RECONNECT"),
            HeaderType::Ping => String::from("PING"),

            HeaderType::PlayCard => String::from("PLAY_CARD"),
            HeaderType::AttackPlayer => String::from("ATTACK_PLAYER"),

            HeaderType::InvalidHeader => String::from("INVALID_HEADER"),
            HeaderType::AlreadyConnected => String::from("ALREADY_CONNECTED"),
            HeaderType::InvalidPlayerData => String::from("INVALID_PLAYER_DATA"),
            HeaderType::InvalidChecksum => String::from("INVALID_CHECKSUM"),
            HeaderType::FailedToConnectPlayer => String::from("FAILED_TO_CONNECT_PLAYER"),
            HeaderType::InvalidPacketPayload => String::from("INVALID_PACKET_PAYLOAD"),
            HeaderType::ERROR => String::from("ERROR"),

            HeaderType::GameState => String::from("GAME_STATE"),
        };
        
        write!(f, "{}", str)
    }
}

impl TryFrom<u8> for HeaderType {
    type Error = ();

    /// Attempts to convert a `u8` into a `HeaderType`.
    ///
    /// Returns `Ok(HeaderType)` if the byte matches a known variant.
    /// Returns `Err(())` if the byte does not correspond to any defined message type.
    ///
    /// Useful for deserializing incoming packets.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(HeaderType::Disconnect),
            0x01 => Ok(HeaderType::Connect),
            0x02 => Ok(HeaderType::Ping),
            0x03 => Ok(HeaderType::Reconnect),

            0x10 => Ok(HeaderType::GameState),
            0x11 => Ok(HeaderType::PlayCard),
            0x12 => Ok(HeaderType::AttackPlayer),

            0xFA => Ok(HeaderType::InvalidHeader),
            0xFB => Ok(HeaderType::AlreadyConnected),
            0xFC => Ok(HeaderType::InvalidPlayerData),
            0xFD => Ok(HeaderType::InvalidChecksum),
            0xF0 => Ok(HeaderType::FailedToConnectPlayer),
            0xF1 => Ok(HeaderType::InvalidPacketPayload),
            0xFE => Ok(HeaderType::ERROR),
            _ => Err(()),
        }
    }
}

/// Represents a fixed-size protocol header for game packet transmission.
///
/// Contains the message type, payload length, and a checksum for validation.
/// Serialized as 6 bytes total when sent over the network.
#[derive(Clone)]
pub struct Header {
    pub checksum: i16,
    pub payload_length: i16,
    pub header_type: HeaderType,
}

impl Header {
    /// Creates a new `PacketHeader` from the given message type and payload.
    ///
    /// Calculates the checksum and payload length automatically.
    pub fn new(header_type: HeaderType, payload: &[u8]) -> Self {
        Self {
            checksum: CheckSum::new(payload) as i16,
            payload_length: payload.len() as i16,
            header_type,
        }
    }

    /// Serializes the header into a fixed-size byte array.
    ///
    /// Format: [type, payload_len (2 bytes), checksum (2 bytes), 0x0A].
    pub fn wrap_header(&self) -> Box<[u8]> {
        let checksum: u16 = self.checksum as u16;
        let payload_length: u16 = self.payload_length as u16;
        let header_type: u8 = self.header_type.to_owned() as u8;

        Box::new([
            header_type,
            ((payload_length >> 8) & 0xFF) as u8,
            (payload_length & 0xFF) as u8,
            ((checksum >> 8) & 0xFF) as u8,
            (checksum & 0xFF) as u8,
            0x0A,
        ])
    }

    /// Parses a `PacketHeader` from a byte slice.
    ///
    /// Returns an error if the slice is too short or has an invalid type.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() != 6 || bytes[5] != 0x0A {
            return Err(ProtocolError::InvalidHeaderError(format!(
                "Format invalid: {:?}",
                bytes
            )));
        }

        match HeaderType::try_from(bytes[0]) {
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
        }
    }
}
