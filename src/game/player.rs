use crate::utils::errors::InvalidPlayerPayload;

pub struct Player {
    pub uuid: String,
}

impl Player {
    pub fn new(payload: &[u8]) -> Result<Self, InvalidPlayerPayload> {
        if let Ok(str) = std::str::from_utf8(payload) {
            let split: Vec<&str> = str.split("\n").collect();
            if split.len() < 2 {
                return Err(InvalidPlayerPayload);
            }

            return Ok(Self {
                uuid: split[0].to_owned(),
            });
        } else {
            return Err(InvalidPlayerPayload);
        }
    }
}
