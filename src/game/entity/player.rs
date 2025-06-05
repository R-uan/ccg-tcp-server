use std::sync::Arc;
use crate::models::client_requests::ReconnectionRequest;
use crate::models::http_response::AuthenticatedPlayer;
use crate::{logger, models::{client_requests::ConnectionRequest, http_response::PartialPlayerProfile}, utils::{errors::PlayerConnectionError, logger::Logger}, SETTINGS};
use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};
use crate::game::entity::deck::Deck;
use crate::game::entity::board::{BoardView, GraveyardView};
use crate::game::entity::card::CardView;

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
        match serde_cbor::from_slice::<ConnectionRequest>(payload) {
            Err(error) => Err(PlayerConnectionError::InvalidPlayerPayload(error.to_string())),
            Ok(request) => {
                let player_profile = Player::get_player_profile(&request.auth_token).await?;
                logger!(INFO, "Fetched `{}`'s profile", &player_profile.username);
                
                let player_deck = Player::get_player_deck(&request.current_deck_id, &request.auth_token).await?;
                logger!(INFO, "Fetched `{}` cards from `{}`'s deck", player_deck.cards.len(), &player_profile.username);
                
                Ok(Player {
                    id: request.player_id,
                    current_deck: player_deck,
                    level: player_profile.level,
                    player_token: request.auth_token,
                    username: player_profile.username,
                    current_deck_id: request.current_deck_id,
                })
            }
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
            Err(e) => Err(PlayerConnectionError::UnexpectedDeckError(e.to_string())),
            Ok(response) => match response.status() {
                StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedDeckError),
                
                StatusCode::NOT_FOUND => Err(PlayerConnectionError::DeckNotFound),
                
                StatusCode::OK => Ok(response
                    .json::<Deck>()
                    .await
                    .map_err(|_| PlayerConnectionError::InvalidDeckFormat)?),
                
                _ => {
                    let error_msg = response.text().await.unwrap_or("NO MESSAGE".to_string());
                    Err(PlayerConnectionError::UnexpectedDeckError(error_msg))
                }
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
            Ok(response) => match response.status()  {
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
    pub health: i32,
    pub mana: i32,

    pub hand_size: usize,
    pub deck_size: usize,
    pub current_hand: [Option<CardView>; 10],

    pub board: BoardView,
    pub graveyard_size: usize,
    pub graveyard: GraveyardView,
}

impl PlayerView {
    pub fn from_player(player: Arc<Player>) -> Self {
        PlayerView {
            id: player.id.clone(),
            health: 30,
            mana: 1,

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