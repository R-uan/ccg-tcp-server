use crate::utils::{checksum::CheckSum, errors::InvalidHeaderError};

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

#[derive(Clone)]
pub struct ProtocolHeader {
    pub checksum: i16,
    pub payload_length: i16,
    pub header_type: MessageType,
}

impl ProtocolHeader {
    /**
        Creates a new ProtocolHeader instance. \
    */
    pub fn new(header_type: MessageType, payload: &[u8]) -> Self {
        return Self {
            checksum: CheckSum::new(payload) as i16,
            payload_length: payload.len() as i16,
            header_type,
        };
    }

    /**
        Turns Self properties into a `Box<[u8]>`
    */
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

    /**
        Creates a ProtocolHeader instance from u8 bytes. \
        Can throw error if:
        * Header contains less than 5 bytes
        * Can not convert u8 into `MessageType`
    */
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

#[derive(Clone)]
pub struct Packet {
    pub header: ProtocolHeader,
    pub payload: Box<[u8]>,
}

impl Packet {
    /**
        Processes the raw tcp byte stream into a Packet instance.\
        Turns the header into a ProtocolHeader while keeping the body bytes \
        as `Box<[u8]>`. \

        This function is used on the raw stream data caught from the TcpStream.
    */
    pub fn parse(protocol: &[u8]) -> Result<Self, InvalidHeaderError> {
        let header = ProtocolHeader::from_bytes(&protocol[..5])?;
        let payload = protocol[6..].to_owned().into_boxed_slice();
        return Ok(Self { header, payload });
    }

    /**
        Creates a Packet instance from a `MessageType` and `payload`.\
        This function is used to create a new packet that most likely will be sent
        back to the client.
    */
    pub fn new(header_type: MessageType, payload: &[u8]) -> Self {
        let header = ProtocolHeader::new(header_type, payload);
        let payload = payload.to_vec().into_boxed_slice();
        return Self { header, payload };
    }

    /**
        Wraps the `header` and the `payload` into a `Box<[u8]>`, ready to be sent through
        the client stream.
    */
    pub fn wrap_packet(&self) -> Box<[u8]> {
        let header = self.header.wrap_header();
        let mut packet = Vec::with_capacity(header.len() + self.payload.len());

        packet.extend_from_slice(&header);
        packet.extend_from_slice(&self.payload);

        packet.into_boxed_slice()
    }
}
