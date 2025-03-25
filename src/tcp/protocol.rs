use std::net::SocketAddr;

use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::MutexGuard};

use crate::utils::{
    checksum::CheckSum,
    errors::{InvalidHeaderError, PackageWriteError},
    logger::Logger,
};

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolType {
    Close = 0x00,
    Connect = 0x01,
    GameState = 0x02,

    PConnect = 0x10,
    PDisconnect = 0x11,
    PMovement = 0x12,
    PAttack = 0x13,

    Err = 0xFF,
}

impl TryFrom<u8> for ProtocolType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(ProtocolType::Close),
            0x01 => Ok(ProtocolType::Connect),
            0x02 => Ok(ProtocolType::GameState),
            0x10 => Ok(ProtocolType::PConnect),
            0x11 => Ok(ProtocolType::PDisconnect),
            0x12 => Ok(ProtocolType::PMovement),
            0x13 => Ok(ProtocolType::PAttack),
            _ => Err(()),
        }
    }
}

pub struct ProtocolHeader {
    pub checksum: i16,
    pub payload_length: i16,
    pub header_type: ProtocolType,
}

impl ProtocolHeader {
    pub fn new(header_type: ProtocolType, payload: &[u8]) -> Self {
        return Self {
            checksum: CheckSum::new(payload) as i16,
            payload_length: payload.len() as i16,
            header_type,
        };
    }

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

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidHeaderError> {
        if bytes.len() < 5 {
            return Err(InvalidHeaderError);
        }

        match ProtocolType::try_from(bytes[0]) {
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

pub struct Packet {
    pub header: ProtocolHeader,
    pub payload: Box<[u8]>,
}

impl Packet {
    pub fn parse(protocol: &[u8]) -> Result<Self, InvalidHeaderError> {
        let header = ProtocolHeader::from_bytes(&protocol[..5])?;
        let payload = protocol[6..].to_owned().into_boxed_slice();
        return Ok(Self { header, payload });
    }

    pub fn new(header_type: ProtocolType, payload: &[u8]) -> Self {
        let header = ProtocolHeader::new(header_type, payload);
        let payload = payload.to_vec().into_boxed_slice();
        return Self { header, payload };
    }

    pub fn wrap_packet(&self) -> Box<[u8]> {
        let header = self.header.wrap_header();
        let mut packet = Vec::with_capacity(header.len() + self.payload.len());

        packet.extend_from_slice(&header);
        packet.extend_from_slice(&self.payload);

        packet.into_boxed_slice()
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
