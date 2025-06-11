use std::sync::Arc;
use crate::models::client_requests::ReconnectionRequest;
use crate::models::http_response::{AuthenticatedPlayer, PreloadedPlayer};
use crate::{logger, models::{http_response::PartialPlayerProfile}, utils::{errors::PlayerConnectionError, logger::Logger}, SETTINGS};
use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::game::entity::deck::{Deck, DeckView};
use crate::game::entity::board::{BoardView, GraveyardView};
use crate::game::entity::card::{CardRef, CardView};

/// Represents a player in the game, including their profile, deck, and authentication details.
#[derive(Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub current_deck: Deck,
    pub deck_view: DeckView,
    pub current_deck_id: String,
}

impl Player {
    pub async fn preload_player(profile: PreloadedPlayer, deck: Deck, deck_view: DeckView) -> Self {
        Player {
            deck_view,
            id: profile.id,
            level: profile.level,
            username: profile.username,
            current_deck_id: deck.id.clone(),
            current_deck:deck,
        }
    }

    pub async fn preload_player_profile(player_id: &str) -> Result<PreloadedPlayer, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/preload/{player_id}", settings.auth_server);
        let reqwest_client = reqwest::Client::new();

        match reqwest_client.get(api_url).send().await {
            Ok(response) => {
                Ok(response.json::<PreloadedPlayer>().await.map_err(|e| {
                    PlayerConnectionError::InvalidPlayerPayload(e.to_string())
                })?)
            }
            Err(error) => Err(PlayerConnectionError::UnexpectedDeckError(error.to_string()))?
        }
    }

    pub async fn preload_player_deck(deck_id: &str) -> Result<Deck, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/deck/{}", settings.deck_server, deck_id);
        let reqwest_client = reqwest::Client::new();

        match reqwest_client.get(api_url).send().await {
            Err(e) => Err(PlayerConnectionError::UnexpectedDeckError(e.to_string())),
            Ok(response) => match response.status() {
                StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedDeckError),

                StatusCode::NOT_FOUND => Err(PlayerConnectionError::DeckNotFound),

                StatusCode::OK => {
                    let deck = response
                        .json::<Deck>()
                        .await
                        .map_err(|_| PlayerConnectionError::InvalidDeckFormat)?;

                    Ok(deck)
                }

                _ => {
                    let error_msg = response.text().await.unwrap_or("NO MESSAGE".to_string());
                    Err(PlayerConnectionError::UnexpectedDeckError(error_msg))
                }
            },
        }
    }

    /// Handles player reconnection by verifying the authentication token and matching the player ID.
    ///
    /// # Arguments
    /// * `payload` - A byte slice containing the serialized reconnection request.
    ///
    /// # Returns
    /// * `Ok(AuthenticatedPlayer)` - The authenticated player instance.
    /// * `Err(PlayerConnectionError)` - An error if the payload is invalid or authentication fails.
    pub async fn reconnection(payload: &[u8]) -> Result<AuthenticatedPlayer, PlayerConnectionError> {
        match serde_cbor::from_slice::<ReconnectionRequest>(payload) {
            Err(error) => Err(PlayerConnectionError::InvalidPlayerPayload(error.to_string())),
            Ok(request) => {
                let player_profile = Player::verify_authentication(&request.auth_token).await?;
                if player_profile.player_id != request.player_id {
                    return Err(PlayerConnectionError::PlayerDiscrepancy);
                }

                Ok(player_profile)
            }
        }
    }

    /// Verifies the player's authentication token by contacting the authentication server.
    ///
    /// # Arguments
    /// * `token` - The authentication token to verify.
    ///
    /// # Returns
    /// * `Ok(AuthenticatedPlayer)` - The authenticated player details.
    /// * `Err(PlayerConnectionError)` - An error if the token is invalid or the server response is unexpected.
    async fn verify_authentication(token: &str) -> Result<AuthenticatedPlayer, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/auth/verify", settings.auth_server);
        let reqwest_client = reqwest::Client::new();

        match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Err(error) => Err(PlayerConnectionError::UnexpectedPlayerError(error.to_string())),
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let result = response.json::<AuthenticatedPlayer>().await.map_err(|e| {
                        logger!(ERROR, "{}", e.to_string());
                        PlayerConnectionError::InvalidResponseBody("AuthenticatedPlayer".to_string())
                    })?;

                    if result.is_banned == true {
                        return Err(PlayerConnectionError::BannedPlayer(result.username.to_string()));
                    }

                    Ok(result)
                }
                StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                _ => Err(PlayerConnectionError::UnexpectedPlayerError(format!(
                    "Unexpected authentication response status: {}",
                    &response.status()
                ))),
            },
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
    async fn get_player_profile(token: &str) -> Result<PartialPlayerProfile, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/account", settings.auth_server);
        let reqwest_client = reqwest::Client::new();
        match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Err(e) => Err(PlayerConnectionError::UnexpectedDeckError(e.to_string())),
            Ok(response) => match response.status() {
                StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                StatusCode::OK => response.json::<PartialPlayerProfile>().await.map_err(|e| {
                    PlayerConnectionError::InvalidPlayerPayload(e.to_string())
                }),
                _ => {
                    let error_msg = response.text().await.unwrap_or("NO MESSAGE".to_string());
                    Err(PlayerConnectionError::UnexpectedDeckError(error_msg))
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerView {
    pub id: String,
    pub mana: i32,
    pub health: i32,

    pub hand_size: usize,
    pub deck_size: usize,
    pub current_hand: [Option<CardView>; 10],

    pub board: BoardView,
    pub graveyard_size: usize,
    pub graveyard: GraveyardView,
}

impl PlayerView {
    pub fn from_player(player: &Player) -> Self {
        PlayerView {
            mana: 1,
            health: 30,
            id: player.id.clone(),

            hand_size: 0,
            graveyard_size: 0,
            board: BoardView::default(),
            graveyard: GraveyardView::default(),
            deck_size: player.current_deck.cards.len(),
            current_hand: [None, None, None, None, None, None, None, None, None, None],
        }
    }
}

#[derive(Serialize, Clone)]
pub struct PublicPlayerView {
    pub id: String,
    pub health: i32,
    pub mana: i32,
    pub hand_size: usize,
    pub deck_size: usize,
    pub graveyard_size: usize,
    pub board: BoardView,
}