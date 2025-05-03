use crate::{
    models::{client_requests::ConnRequest, deck::Deck, http_response::PartialPlayerProfile},
    utils::{errors::PlayerConnectionError, logger::Logger},
    SETTINGS,
};
use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub player_color: String,
    pub player_token: String,
    pub current_deck: Deck,
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
    pub async fn new(payload: &[u8]) -> Result<Self, PlayerConnectionError> {
        match serde_cbor::from_slice::<ConnRequest>(payload) {
            Err(error) => {
                let reason = error.to_string();
                Logger::error(&format!("{}", &reason));
                return Err(PlayerConnectionError::InvalidPlayerPayload(reason));
            }
            Ok(request) => {
                let player_profile = Player::get_player_profile(&request.token).await?;
                let player_deck =
                    Player::get_player_deck(&request.current_deck_id, &request.token).await?;
                Logger::info(&format!(
                    "{}: Fetched deck with: {} cards",
                    &request.id,
                    player_deck.cards.len()
                ));

                return Ok(Player {
                    id: request.id,
                    current_deck: player_deck,
                    player_token: request.token,
                    level: player_profile.level,
                    username: player_profile.username,
                    player_color: request.player_color,
                    current_deck_id: request.current_deck_id,
                });
            }
        }
    }

    async fn get_player_deck(deck_id: &str, token: &str) -> Result<Deck, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/deck/{}", settings.deck_server, deck_id);
        let reqwest_client = reqwest::Client::new();
        Logger::debug(deck_id);
        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let result = response
                        .json::<Deck>()
                        .await
                        .map_err(|_| PlayerConnectionError::InvalidDeckFormat);
                    result
                }
                StatusCode::NOT_FOUND => Err(PlayerConnectionError::DeckNotFound),
                _ => {
                    let error_msg = response.text().await.unwrap();
                    Logger::error(&error_msg);
                    Err(PlayerConnectionError::UnexpectedDeckError)
                }
            },
            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedDeckError),
                    _ => Err(PlayerConnectionError::UnexpectedDeckError),
                };
            }
        };
    }

    async fn get_player_profile(
        token: &str,
    ) -> Result<PartialPlayerProfile, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/profile", settings.auth_server);
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
                    .map_err(|e| PlayerConnectionError::InvalidPlayerPayload(e.to_string()));
                result
            }

            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                Logger::error("Player profile fetch error");
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                    _ => Err(PlayerConnectionError::UnexpectedPlayerError),
                };
            }
        };
    }
}
