use crate::{models::card::Card, utils::errors::PlayerErrors};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub player_token: String,
    pub current_deck_id: String,
    pub current_deck: Option<Vec<Card>>,
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
    pub async fn new(payload: &[u8]) -> Result<Self, PlayerErrors> {
        let mut player: Player = serde_cbor::from_slice::<Player>(payload)
            .map_err(|_| PlayerErrors::InvalidPlayerPayload)?;

        match reqwest::get(format!("/api/player/deck/{}", player.current_deck_id)).await {
            Ok(response) => {
                let deck = response
                    .json::<Vec<Card>>()
                    .await
                    .map_err(|_| PlayerErrors::InvalidDeckError)?;

                player.current_deck = Some(deck);
            }
            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerErrors::UnauthorizedPlayerError),
                    _ => Err(PlayerErrors::UnexpectedPlayerError),
                };
            }
        };

        return Ok(player);
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_valid_player_creation() {
//         let player_data = &Player {
//             id: "1a2b3c4d".to_string(),
//             username: "Tester".to_string(),
//             current_deck_id: "objectid-of-the-deck".to_string(),
//             level: 50,
//             current_deck: None,
//             player_token: "".to_string(),
//         };

//         let player_bytes = serde_cbor::to_vec(player_data).unwrap();

//         let result = Player::new(&player_bytes);
//         assert!(result.is_ok());

//         let player = result.unwrap();

//         assert_eq!(player.id, player_data.id);
//         assert_eq!(player.username, player_data.username);
//         assert_eq!(player.current_deck_id, player_data.current_deck_id);
//         assert_eq!(player.level, player_data.level);
//     }

//     #[test]
//     fn test_invalid_player_creation() {
//         let bad_payload = b"lol\nwhat\nisthis";
//         let result = Player::new(bad_payload);
//         assert!(result.is_err());
//         match result {
//             Err(e) => assert_eq!(e, InvalidPlayerPayload),
//             Ok(_) => panic!("Expected error, got Ok"),
//         };
//     }
// }
