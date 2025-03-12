use std::io::Error;

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        return Self { x, y };
    }
}

pub struct PlayerState {
    pub id: String,
    pub position: Vec2,
    pub nickname: String,
}

impl PlayerState {
    /**
    # Arguments
    * `body`: An array of u8 bytes that contains the valid protocol body for a player connection request.

    # Returns
    * PlayerState with the data extracted from the protocol.
    */
    pub fn new(body: &[u8]) -> Result<Self, Error> {
        let fields: Vec<&str> = std::str::from_utf8(body)
            .expect("Couldn't read bytes")
            .split("\n")
            .collect();

        if fields.len() < 2 {
            return Err(Error::last_os_error());
        }

        return Ok(Self {
            id: fields[0].to_owned(),
            nickname: fields[1].to_owned(),
            position: Vec2::new(0.0, 0.0),
        });
    }
}
