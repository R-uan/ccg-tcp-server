use std::fmt::{self};

pub struct Protocol {}
pub struct CheckSum {}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum ProtocolType {
    Close = 0x00,
    Connect = 0x01,
    GameState = 0x02,

    PlayerConnected = 0x10,
    PlayerMovement = 0x11,

    Err = 0xFF,
}

impl TryFrom<u8> for ProtocolType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(ProtocolType::Close),
            0x01 => Ok(ProtocolType::Connect),
            0x02 => Ok(ProtocolType::GameState),
            0x03 => Ok(ProtocolType::PlayerConnected),
            0x10 => Ok(ProtocolType::PlayerMovement),
            _ => Err(()),
        }
    }
}

pub struct ProtocolHeader {
    pub checksum: i16,
    pub message_length: i16,
    pub operation: ProtocolType,
}

#[derive(Debug)]
pub struct InvalidHeaderError;

impl fmt::Display for InvalidHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid protocol header")
    }
}

impl ProtocolHeader {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidHeaderError> {
        if bytes.len() < 5 {
            return Err(InvalidHeaderError);
        }

        match ProtocolType::try_from(bytes[0]) {
            Err(_) => return Err(InvalidHeaderError),
            Ok(h_type) => {
                let c_sum: i16 = u16::from_be_bytes([bytes[3], bytes[4]]) as i16;
                let m_length: i16 = u16::from_be_bytes([bytes[1], bytes[2]]) as i16;

                return Ok(Self {
                    operation: h_type,
                    message_length: m_length,
                    checksum: c_sum,
                });
            }
        }
    }

    pub fn new(operation: ProtocolType, payload: &[u8]) -> Vec<u8> {
        let header_type = operation as u8;
        let payload_len = payload.len() as u16;
        let checksum = CheckSum::new(payload);

        return vec![
            header_type,
            ((payload_len >> 8) & 0xFF) as u8,
            (payload_len & 0xFF) as u8,
            ((checksum >> 8) & 0xFF) as u8,
            (checksum & 0xFF) as u8,
            0x0A,
        ];
    }
}

impl Protocol {
    pub fn create_packet(operation: ProtocolType, payload: &[u8]) -> Box<[u8]> {
        let mut header = ProtocolHeader::new(operation, payload);
        let payload = payload.to_vec();
        header.extend_from_slice(&payload);
        let bytes: Box<[u8]> = header.into_boxed_slice();
        return bytes;
    }
}

impl CheckSum {
    pub fn new(payload: &[u8]) -> u16 {
        let mut checksum: u16 = 0;
        for &byte in payload {
            checksum ^= byte as u16;
        }

        return checksum;
    }

    pub fn check(checksum: &i16, payload: &[u8]) -> bool {
        let check = CheckSum::new(payload);
        return *checksum == check as i16;
    }
}
