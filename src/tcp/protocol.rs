use std::fmt::{self};

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum ProtocolOperations {
    Close = 0x00,
    Connect = 0x01,
    Update = 0x02,

    PlayerConnected = 0x10,
    PlayerMovement = 0x11,

    Err = 0xFF,
}

impl TryFrom<u8> for ProtocolOperations {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(ProtocolOperations::Close),
            0x01 => Ok(ProtocolOperations::Connect),
            0x02 => Ok(ProtocolOperations::Update),
            0x03 => Ok(ProtocolOperations::PlayerConnected),
            0x10 => Ok(ProtocolOperations::PlayerMovement),
            _ => Err(()),
        }
    }
}

pub struct ProtocolHeader {
    pub checksum: i16,
    pub message_length: i16,
    pub operation: ProtocolOperations,
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

        match ProtocolOperations::try_from(bytes[0]) {
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

    pub fn new(h_type: ProtocolOperations, body: &[u8]) -> Vec<u8> {
        let header_type = h_type as u8;
        let b_len = body.len() as u16;
        let checksum = CheckSum::new(body);

        return vec![
            header_type,
            (b_len >> 8) as u8,
            (b_len & 0xFF) as u8,
            ((checksum >> 8) & 0xFF) as u8,
            (checksum & 0xFF) as u8,
            0x0A,
        ];
    }
}

pub struct Protocol;
impl Protocol {
    pub fn create_packet(h_type: ProtocolOperations, body: &[u8]) -> Box<[u8]> {
        let mut header = ProtocolHeader::new(h_type, body);
        let mut v_body = body.to_vec();
        v_body.push(0x1A);
        header.extend_from_slice(&v_body);
        let slice: Box<[u8]> = header.into_boxed_slice();
        return slice;
    }
}

pub struct CheckSum;

impl CheckSum {
    pub fn new(data: &[u8]) -> u16 {
        let mut checksum: u16 = 0;
        for &byte in data {
            checksum ^= byte as u16;
        }
        return checksum;
    }

    pub fn check(checksum: &i16, data: &[u8]) -> bool {
        let check = CheckSum::new(data);
        println!("[Info] # Comparing checksum: {check} : {checksum}");
        return *checksum == check as i16;
    }
}
