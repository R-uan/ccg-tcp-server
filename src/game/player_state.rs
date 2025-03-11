use std::io::{BufRead, BufReader, Error};

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
    pub fn forge_connection(p_body: &[u8]) -> Result<Self, Error> {
        let fields: Vec<&str> = std::str::from_utf8(p_body)
            .expect("Couldn't read bytes")
            .split("\n")
            .collect();

        if fields.len() < 2 {
            return Err(Error::last_os_error());
        }

        println!("{:?}", fields);

        return Ok(Self {
            id: fields[0].to_owned(),
            nickname: fields[1].to_owned(),
            position: Vec2::new(0.0, 0.0),
        });
    }
}
