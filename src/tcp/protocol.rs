use crate::utils::{checksum::CheckSum, errors::InvalidHeaderError};

/// Represents the type of message in a protocol packet.
///
/// Each variant maps to a specific `u8` value used during transmission.
///
/// # Variants
///
/// - `DISCONNECT` - Client is disconnecting.
/// - `CONNECT` - Client is initiating a connection.
/// - `GAMESTATE` - Server is sending current game state.
///
/// ### Errors (0xFB–0xFF):
/// - `ALREADYCONNECTED` - Client is already connected.
/// - `INVALIDPLAYERDATA` - Malformed or missing player data.
/// - `INVALIDCHECKSUM` - Payload failed checksum validation.
/// - `INVALIDHEADER` - Malformed or unrecognized header.
/// - `ERROR` - Generic error.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    DISCONNECT = 0x00,
    CONNECT = 0x01,

    GAMESTATE = 0x10,

    ALREADYCONNECTED = 0xFB,
    INVALIDPLAYERDATA = 0xFC,
    INVALIDCHECKSUM = 0xFD,
    INVALIDHEADER = 0xFE,
    ERROR = 0xFF,
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
            0x00 => Ok(MessageType::DISCONNECT),
            0x01 => Ok(MessageType::CONNECT),
            0x02 => Ok(MessageType::GAMESTATE),
            0xFF => Ok(MessageType::ERROR),
            _ => Err(()),
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
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidHeaderError> {
        if bytes.len() < 5 {
            return Err(InvalidHeaderError);
        }

        match MessageType::try_from(bytes[0]) {
            Err(_) => return Err(InvalidHeaderError),
            Ok(header_type) => {
                let checksum: i16 = u16::from_be_bytes([bytes[3], bytes[4]]) as i16;
                let payload_length: i16 = u16::from_be_bytes([bytes[1], bytes[2]]) as i16;

                return Ok(Self {
                    header_type,
                    payload_length,
                    checksum,
                });
            }
        }
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
    pub fn parse(protocol: &[u8]) -> Result<Self, InvalidHeaderError> {
        let header = ProtocolHeader::from_bytes(&protocol[..5])?;
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
