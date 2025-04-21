use crate::{
    models::{card::Card, client_requests::ConnRequest, http_response::PartialPlayerProfile},
    utils::{errors::PlayerErrors, logger::Logger},
    SETTINGS,
};
use reqwest::{header::AUTHORIZATION, StatusCode};
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
        let request: ConnRequest =
            serde_cbor::from_slice(payload).map_err(|_| PlayerErrors::InvalidPlayerPayload)?;
        let player_profile = Player::get_player_profile(&request.token).await?;
        let player_deck = Player::get_player_deck(&request.current_deck_id, &request.token).await?;

        return Ok(Player {
            id: request.id,
            player_token: request.token,
            level: player_profile.level,
            username: player_profile.username,
            current_deck_id: request.current_deck_id,
            current_deck: Some(player_deck),
        });
    }

    async fn get_player_deck(deck_id: &str, token: &str) -> Result<Vec<Card>, PlayerErrors> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/deck/{}", settings.deck_server, deck_id);
        let reqwest_client = reqwest::Client::new();

        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                let result = response
                    .json::<Vec<Card>>()
                    .await
                    .map_err(|_| PlayerErrors::InvalidDeckError);
                result
            }
            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerErrors::UnauthorizedPlayerError),
                    _ => Err(PlayerErrors::UnexpectedPlayerError),
                };
            }
        };
    }

    async fn get_player_profile(token: &str) -> Result<PartialPlayerProfile, PlayerErrors> {
        let api_url = format!("http://127.0.0.1:5001/api/player/profile");
        let reqwest_client = reqwest::Client::new();

        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                let result = response
                    .json::<PartialPlayerProfile>()
                    .await
                    .map_err(|_| PlayerErrors::InvalidPlayerPayload);
                result
            }

            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                Logger::error("Player profile fetch error");
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerErrors::UnauthorizedPlayerError),
                    _ => Err(PlayerErrors::UnexpectedPlayerError),
                };
            }
        };
    }
}
