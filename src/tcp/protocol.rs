use std::io::Error;

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum HeaderTypes {
    Close = 0x00,
    Connect = 0x01,
    Update = 0x02,
    PlayerConnected = 0x03,
    Err = 0xFF,
}

impl TryFrom<u8> for HeaderTypes {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(HeaderTypes::Connect),
            0x02 => Ok(HeaderTypes::Update),
            _ => Err(()),
        }
    }
}

pub struct PacketHeader {
    message_type: u8,
    message_length: u16,
    checksum: u16,
}

impl PacketHeader {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 5 {
            return None;
        }

        return Some(Self {
            message_type: bytes[0],
            message_length: u16::from_be_bytes([bytes[1], bytes[2]]),
            checksum: u16::from_be_bytes([bytes[3], bytes[4]]),
        });
    }

    pub fn convert(&self) -> Result<(HeaderTypes, i16, i32), Error> {
        let h_type = HeaderTypes::try_from(self.message_type).expect("Invalid header type");
        let m_length: i16 = self.message_length as i16;
        let c_sum: i32 = self.checksum as i32;
        return Ok((h_type, m_length, c_sum));
    }

    pub fn new(h_type: HeaderTypes, body: &[u8]) -> Vec<u8> {
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
    pub fn create_response(h_type: HeaderTypes, body: &[u8]) -> Box<[u8]> {
        let mut header = PacketHeader::new(h_type, body);
        let v_body = body.to_vec();
        header.extend_from_slice(&v_body);
        let slice: Box<[u8]> = header.into_boxed_slice();
        return slice;
    }
}

struct CheckSum;

impl CheckSum {
    pub fn new(data: &[u8]) -> u16 {
        let mut checksum: u16 = 0;
        for &byte in data {
            checksum ^= byte as u16;
        }
        return checksum;
    }
}
