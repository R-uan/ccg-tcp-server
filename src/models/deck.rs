use crate::models::http_response::SelectedCardsResponse;
use crate::utils::errors::CardRequestError;
use crate::utils::errors::CardRequestError::CardNotFound;
use crate::utils::logger::Logger;
use crate::SETTINGS;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Deck {
    pub id: String,
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub cards: Vec<CardRef>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CardRef {
    pub id: String,
    pub amount: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub description: String,
    pub play_cost: u32,
    pub attack: u32,
    pub health: u32,
    pub rarity: u16,

    // These will contain lua function names I guess
    pub on_play: Vec<String>,
    pub on_draw: Vec<String>,

    pub on_attack: Vec<String>,
    pub on_hit: Vec<String>,

    pub on_turn_start: Vec<String>,
    pub on_turn_end: Vec<String>,

    pub on_death: Vec<String>,
    pub on_ally_death: Vec<String>,
    pub on_enemy_death: Vec<String>,
}

impl Card {
    /// Request the CARD_SERVER for one card by ID
    /// Should not require authentication, so the only response possible are errors or OK and NOT FOUND
    pub async fn request_card(card_id: &str) -> Result<Card, CardRequestError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/card/{}", settings.card_server, card_id);
        match reqwest::get(api_url).await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let card = response.json::<Card>().await.map_err(|e| {
                        Logger::error(&format!(
                            "Card `{}` parsing error: {}",
                            card_id,
                            e.to_string()
                        ));
                        return CardRequestError::UnexpectedCardRequestError(
                            "Unable to parse card response".to_string(),
                        );
                    })?;

                    Ok(card)
                }
                StatusCode::NOT_FOUND => {
                    Logger::warn(&format!("Card `{}` was not found when requested.", card_id));
                    Err(CardNotFound(card_id.to_string()))
                }
                _ => {
                    let status = response.status().clone();
                    let response_body = response.text().await.unwrap_or_default();
                    Logger::warn(&format!(
                        "Unexpected card request response {{ status: {}, message: {} }}",
                        status, response_body
                    ));

                    Err(CardRequestError::UnexpectedCardRequestError(response_body))
                }
            },
            Err(error) => {
                Logger::error(&format!(
                    "Unexpected card request error {{ status: {}, message: {} }}",
                    error.status().unwrap_or_default(),
                    error.to_string()
                ));

                Err(CardRequestError::UnexpectedCardRequestError(
                    error.status().unwrap().to_string(),
                ))
            }
        }
    }
    
    pub async fn request_cards(cards: &Vec<CardRef>) -> Result<Vec<Card>, CardRequestError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/card/selected", settings.card_server);
        let card_ids: Vec<&String> = cards.iter().map(|c| &c.id).collect();
        let client = reqwest::Client::new();
        let body = serde_json::json!({"cardIds": card_ids});
        match client.post(api_url).json(&body).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let selected_cards =
                        response
                            .json::<SelectedCardsResponse>()
                            .await
                            .map_err(|_| {
                                Logger::error("Unable to parse card selection response");
                                return CardRequestError::SelectedCardsParseError;
                            })?;

                    if selected_cards.cards_not_found.len() != 0
                        || selected_cards.invalid_card_guid.len() != 0
                    {
                        Logger::error(&format!(
                            "Selected cards not found: {:?}",
                            selected_cards.cards_not_found
                        ));
                        Logger::error(&format!(
                            "Invalid card guid: {:?}",
                            selected_cards.invalid_card_guid
                        ));
                        return Err(CardRequestError::FailedToGetFullCardsData);
                    }

                    Ok(selected_cards.cards)
                }
                _ => {
                    let status = response.status().clone();
                    let response_body = response.text().await.unwrap_or_default();
                    Logger::warn(&format!(
                        "Unexpected card request response: {{ status: {} body: {} }}",
                        status
                        response_body.clone()
                    ));
                    Err(CardRequestError::UnexpectedCardRequestError(response_body))
                }
            },
            Err(e) => {
                let status = e.status().unwrap_or_default();
                Logger::error(&format!(
                    "Card request error: {{ status: {} body: {} }}",
                    status,
                    e.to_string()
                ));
                Err(CardRequestError::UnexpectedCardRequestError(e.to_string()))
            }
        }
    }
}
