use crate::utils::errors::InvalidPlayerPayload;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub current_deck_id: String,
}

impl Player {
    /// Attempts to construct a `Player` from a UTF-8-encoded payload.
    ///
    /// Expects the payload to be a newline-delimited string with the following format:
    /// ```
    /// <id>
    /// <username>
    /// <current_deck_id>
    /// <level>
    /// ```
    ///
    /// Returns:
    /// - `Ok(Player)` if parsing succeeds
    /// - `Err(InvalidPlayerPayload)` if UTF-8 is invalid or format is incorrect
    pub fn new(payload: &[u8]) -> Result<Self, InvalidPlayerPayload> {
        return serde_cbor::from_slice(payload).map_err(|_| InvalidPlayerPayload);
    }
}

#[test]
fn test_valid_player_creation() {
    let player_data = &Player {
        id: "1a2b3c4d".to_string(),
        username: "Tester".to_string(),
        current_deck_id: "objectid-of-the-deck".to_string(),
        level: 50,
    };

    let player_bytes = serde_cbor::to_vec(player_data).unwrap();

    let result = Player::new(&player_bytes);
    assert!(result.is_ok());

    let player = result.unwrap();

    assert_eq!(player.id, player_data.id);
    assert_eq!(player.username, player_data.username);
    assert_eq!(player.current_deck_id, player_data.current_deck_id);
    assert_eq!(player.level, player_data.level);
}

#[test]
fn test_invalid_player_creation() {
    let bad_payload = b"lol\nwhat\nisthis";
    let result = Player::new(bad_payload);
    assert!(result.is_err());
    match result {
        Err(e) => assert_eq!(e, InvalidPlayerPayload),
        Ok(_) => panic!("Expected error, got Ok"),
    };
}
