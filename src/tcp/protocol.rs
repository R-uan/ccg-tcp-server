use std::net::SocketAddr;

use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::MutexGuard};

use crate::utils::{
    checksum::CheckSum,
    errors::{InvalidHeaderError, PackageWriteError},
    logger::Logger,
};

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
    pub ptype: ProtocolType,
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
                    ptype: h_type,
                    message_length: m_length,
                    checksum: c_sum,
                });
            }
        }
    }

    pub fn new(ptype: ProtocolType, payload: &[u8]) -> Vec<u8> {
        let header_type = ptype as u8;
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

pub struct Protocol {
    header: Vec<u8>,
    payload: Vec<u8>,
}

impl Protocol {
    pub fn create_packet(ptype: ProtocolType, payload: &[u8]) -> Self {
        let header = ProtocolHeader::new(ptype, payload);
        let payload = payload.to_vec();
        return Protocol { header, payload };
    }

    fn wrap_packet(&self) -> Box<[u8]> {
        let mut package = self.header.clone();
        package.extend_from_slice(&self.payload);
        return package.into_boxed_slice();
    }

    pub async fn send(
        &self,
        mut w_stream: MutexGuard<'_, OwnedWriteHalf>,
        addr: &SocketAddr,
    ) -> Result<(), PackageWriteError> {
        let packet = self.wrap_packet();
        if let Err(_) = w_stream.write_all(&packet).await {
            Logger::error(&format!("{addr}: unable to send packet"));
            return Err(PackageWriteError);
        }
        return Ok(());
    }
}
