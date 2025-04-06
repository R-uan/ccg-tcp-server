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
    PING = 0x02,

    GAMESTATE = 0x10,

    INVALIDHEADER = 0xFA,
    ALREADYCONNECTED = 0xFB,
    INVALIDPLAYERDATA = 0xFC,
    INVALIDCHECKSUM = 0xFD,
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
            0x00 => Ok(MessageType::DISCONNECT),
            0x01 => Ok(MessageType::CONNECT),
            0x10 => Ok(MessageType::GAMESTATE),
            0x02 => Ok(MessageType::PING),
            0xFE => Ok(MessageType::ERROR),
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
        if bytes.len() != 6 || bytes[5] != 0x0A {
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
        if protocol.len() < 6 {
            return Err(InvalidHeaderError);
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
        let header = ProtocolHeader::new(MessageType::PING, payload);

        assert_eq!(header.header_type, MessageType::PING);
        assert_eq!(header.payload_length, 3);
        assert_eq!(header.checksum, CheckSum::new(payload) as i16);
    }

    #[test]
    fn test_protocol_header_wrap_header() {
        let payload = &[0xAA, 0xBB];
        let header = ProtocolHeader::new(MessageType::PING, payload);
        let bytes = header.wrap_header();

        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], MessageType::PING as u8);
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
        let header = ProtocolHeader::new(MessageType::PING, payload);
        let bytes = header.wrap_header();

        let parsed = ProtocolHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header_type, MessageType::PING);
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
        let packet = Packet::new(MessageType::PING, payload);

        assert_eq!(packet.header.header_type, MessageType::PING);
        assert_eq!(packet.header.payload_length, 4);
        assert_eq!(packet.header.checksum, CheckSum::new(payload) as i16);
        assert_eq!(&*packet.payload, payload);
    }

    #[test]
    fn test_packet_wrap_packet() {
        let payload = &[0x42, 0x24];
        let packet = Packet::new(MessageType::PING, payload);
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
        let original = Packet::new(MessageType::PING, payload);
        let raw = original.wrap_packet();

        let parsed = Packet::parse(&raw).unwrap();
        assert_eq!(
            parsed.header.header_type,
            MessageType::PING,
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
