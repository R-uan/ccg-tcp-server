use crate::logger;
use crate::tcp::header::{Header, HeaderType};
use crate::utils::errors::ProtocolError;
use crate::utils::logger::Logger;

/// Represents a complete network packet with a protocol header and payload.
///
/// Handles serialization and parsing for message transmission.
#[derive(Clone)]
pub struct Packet {
    /// The header of the packet, containing metadata such as type and payload length.
    pub header: Header,
    /// The payload of the packet, containing the actual data being transmitted.
    pub payload: Box<[u8]>,
}

impl Packet {
    /// Parses a raw byte slice into a `Packet`.
    ///
    /// Expects a 5-byte header followed by the payload (skips byte 5: delimiter).
    ///
    /// # Arguments
    /// - `protocol`: A byte slice containing the serialized packet data.
    ///
    /// # Returns
    /// - `Ok(Packet)`: If the byte slice is valid and contains a recognizable packet.
    /// - `Err(ProtocolError)`: If the byte slice is invalid or the header cannot be parsed.
    pub fn parse(protocol: &[u8]) -> Result<Self, ProtocolError> {
        if protocol.len() < 6 {
            logger!(ERROR, "[PROTOCOL] Not enough bytes for a valid packet");
            return Err(ProtocolError::InvalidPacketError(
                "Not enough bytes for a valid packet".to_string(),
            ));
        }

        let header = Header::from_bytes(&protocol[..6])?;
        let payload = protocol[6..].to_owned().into_boxed_slice();
        Ok(Self { header, payload })
    }

    /// Creates a new `Packet` from a message type and payload.
    ///
    /// Automatically constructs the header based on the provided payload.
    ///
    /// # Arguments
    /// - `header_type`: The type of the message (e.g., `Connect`, `Disconnect`).
    /// - `payload`: The payload data for the packet.
    ///
    /// # Returns
    /// A new `Packet` instance with the constructed header and payload.
    pub fn new(header_type: HeaderType, payload: &[u8]) -> Self {
        let header = Header::new(header_type, payload);
        let payload = payload.to_vec().into_boxed_slice();
        Self { header, payload }
    }

    /// Serializes the packet into a byte slice.
    ///
    /// Combines the header and payload into a single buffer for transmission.
    ///
    /// # Returns
    /// A boxed array of bytes representing the serialized packet.
    pub fn wrap_packet(&self) -> Box<[u8]> {
        let header = self.header.wrap_header();
        let mut packet = Vec::with_capacity(header.len() + self.payload.len());

        packet.extend_from_slice(&header);
        packet.extend_from_slice(&self.payload);

        packet.into_boxed_slice()
    }
}
