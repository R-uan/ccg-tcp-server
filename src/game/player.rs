use crate::models::client_requests::ReconnectionRequest;
use crate::models::http_response::AuthenticatedPlayer;
use crate::{
    models::{client_requests::ConnectionRequest, deck::Deck, http_response::PartialPlayerProfile},
    utils::{errors::PlayerConnectionError, logger::Logger},
    SETTINGS,
};
use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};

/// Represents a player in the game, including their profile, deck, and authentication details.
#[derive(Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub current_deck: Deck,
    pub player_token: String,
    pub current_deck_id: String,
}

impl Player {
    /// Creates a new player connection by deserializing the payload and fetching the player's profile and deck.
    ///
    /// # Arguments
    /// * `payload` - A byte slice containing the serialized connection request.
    ///
    /// # Returns
    /// * `Ok(Player)` - The newly created player instance.
    /// * `Err(PlayerConnectionError)` - An error if the payload is invalid or the profile/deck fetch fails.
    pub async fn new_connection(payload: &[u8]) -> Result<Self, PlayerConnectionError> {
        return match serde_cbor::from_slice::<ConnectionRequest>(payload) {
            Err(error) => {
                let reason = error.to_string();
                Logger::error(&format!("[PLAYER] {}", &reason));
                Err(PlayerConnectionError::InvalidPlayerPayload(format!(
                    "{reason} (ConnRequest CBOR Deserialisation)"
                )))
            }
            Ok(request) => {
                let player_profile = Player::get_player_profile(&request.auth_token).await?;
                Logger::info(&format!(
                    "[PLAYER] Fetched `{}`'s profile",
                    &player_profile.username
                ));

                let player_deck =
                    Player::get_player_deck(&request.current_deck_id, &request.auth_token).await?;
                Logger::info(&format!(
                    "[PLAYER] Fetched `{}`'s deck with {} cards",
                    &player_profile.username,
                    player_deck.cards.len()
                ));

                Ok(Player {
                    id: request.player_id,
                    current_deck: player_deck,
                    player_token: request.auth_token,
                    level: player_profile.level,
                    username: player_profile.username,
                    current_deck_id: request.current_deck_id,
                })
            }
        };
    }

    /// Handles player reconnection by verifying the authentication token and matching the player ID.
    ///
    /// # Arguments
    /// * `payload` - A byte slice containing the serialized reconnection request.
    ///
    /// # Returns
    /// * `Ok(AuthenticatedPlayer)` - The authenticated player instance.
    /// * `Err(PlayerConnectionError)` - An error if the payload is invalid or authentication fails.
    pub async fn reconnection(
        payload: &[u8],
    ) -> Result<AuthenticatedPlayer, PlayerConnectionError> {
        return match serde_cbor::from_slice::<ReconnectionRequest>(payload) {
            Ok(request) => {
                let player_profile = Player::verify_authentication(&request.auth_token).await?;
                if player_profile.player_id != request.player_id {
                    return Err(PlayerConnectionError::PlayerDoesNotMatch);
                }
                return Ok(player_profile);
            }
            Err(error) => {
                let reason = error.to_string();
                Logger::error(&format!("[PLAYER] {}", &reason));
                Err(PlayerConnectionError::InvalidPlayerPayload(format!(
                    "{reason} (ConnRequest CBOR Deserialisation)"
                )))
            }
        };
    }

    /// Verifies the player's authentication token by contacting the authentication server.
    ///
    /// # Arguments
    /// * `token` - The authentication token to verify.
    ///
    /// # Returns
    /// * `Ok(AuthenticatedPlayer)` - The authenticated player details.
    /// * `Err(PlayerConnectionError)` - An error if the token is invalid or the server response is unexpected.
    async fn verify_authentication(
        token: &str,
    ) -> Result<AuthenticatedPlayer, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/auth/verify", settings.auth_server);
        let reqwest_client = reqwest::Client::new();
        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let result = response.json::<AuthenticatedPlayer>().await.map_err(|_| {
                        PlayerConnectionError::InvalidResponseBody(
                            "AuthenticatedPlayer response was unexpected".to_string(),
                        )
                    })?;

                    if result.is_banned == true {
                        return Err(PlayerConnectionError::PlayerIsBanned);
                    }

                    Ok(result)
                }
                StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                _ => Err(PlayerConnectionError::UnexpectedPlayerError(format!(
                    "Unexpected authentication response status: {}",
                    &response.status()
                ))),
            },
            Err(error) => {
                return Err(PlayerConnectionError::UnexpectedPlayerError(
                    error.to_string(),
                ))
            }
        };
    }

    /// Fetches the player's deck from the deck server using the provided deck ID and authentication token.
    ///
    /// # Arguments
    /// * `deck_id` - The ID of the deck to fetch.
    /// * `token` - The authentication token for the request.
    ///
    /// # Returns
    /// * `Ok(Deck)` - The player's deck.
    /// * `Err(PlayerConnectionError)` - An error if the deck is not found or the server response is invalid.
    async fn get_player_deck(deck_id: &str, token: &str) -> Result<Deck, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/deck/{}", settings.deck_server, deck_id);
        let reqwest_client = reqwest::Client::new();
        match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(response
                    .json::<Deck>()
                    .await
                    .map_err(|_| PlayerConnectionError::InvalidDeckFormat)?),
                StatusCode::NOT_FOUND => Err(PlayerConnectionError::DeckNotFound),
                _ => {
                    let error_msg = response.text().await.unwrap();
                    Logger::error(&format!("[PLAYER] {}", &error_msg));
                    Err(PlayerConnectionError::UnexpectedDeckError)
                }
            },
            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedDeckError),
                    _ => Err(PlayerConnectionError::UnexpectedDeckError),
                }
            }
        }
    }

    /// Fetches the player's profile from the authentication server using the provided token.
    ///
    /// # Arguments
    /// * `token` - The authentication token for the request.
    ///
    /// # Returns
    /// * `Ok(PartialPlayerProfile)` - The player's profile.
    /// * `Err(PlayerConnectionError)` - An error if the profile fetch fails or the response is invalid.
    async fn get_player_profile(
        token: &str,
    ) -> Result<PartialPlayerProfile, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/account", settings.auth_server);
        let reqwest_client = reqwest::Client::new();
        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                // Why is reqwest unauthorized not an error, kinda cringe...
                if response.status() == StatusCode::UNAUTHORIZED {
                    return Err(PlayerConnectionError::UnauthorizedPlayerError);
                }

                let result = response.json::<PartialPlayerProfile>().await.map_err(|_| {
                    PlayerConnectionError::InvalidPlayerPayload(
                        "Failed to deserialize player profile".to_string(),
                    )
                });
                result
            }

            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                Logger::error(&format!("[PLAYER] Profile fetch error ({})", status));
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                    _ => Err(PlayerConnectionError::UnexpectedPlayerError(e.to_string())),
                };
            }
        };
    }
}
