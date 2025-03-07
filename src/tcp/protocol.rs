#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum HeaderTypes {
    Close = 0x00,
    Connect = 0x01,
    Update = 0x02,
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
