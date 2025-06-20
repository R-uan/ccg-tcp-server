use crate::models::http_response::SelectedCardsResponse;
use crate::utils::errors::CardRequestError;
use crate::SETTINGS;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

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
    pub play_cost: i32,
    pub attack: i32,
    pub health: i32,
    pub rarity: i16,

    // These will contain lua function names, I guess
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
    /// Should not require authentication, so the only response possible is errors or OKs and NOT FOUND
    pub async fn request_card(card_id: &str) -> Result<Card, CardRequestError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/card/{}", settings.card_server, card_id);
        match reqwest::get(api_url).await {
            Err(error) => Err(CardRequestError::UnexpectedCardRequestError(
                error.to_string(),
            )),
            Ok(response) => match response.status() {
                StatusCode::NOT_FOUND => Err(CardRequestError::CardNotFound(card_id.to_string())),
                StatusCode::OK => Ok(response.json::<Card>().await.map_err(|e| {
                    return CardRequestError::UnexpectedCardRequestError(e.to_string());
                })?),
                _ => {
                    let response_body = response.text().await.unwrap_or("NO MESSAGE".to_string());
                    Err(CardRequestError::UnexpectedCardRequestError(response_body))
                }
            },
        }
    }

    pub async fn request_cards(cards: &Vec<CardRef>) -> Result<Vec<Card>, CardRequestError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/card/selected", settings.card_server);
        let card_ids: Vec<&String> = cards.iter().map(|c| &c.id).collect();
        let client = reqwest::Client::new();
        let body = serde_json::json!({"cardIds": card_ids});

        match client.post(api_url).json(&body).send().await {
            Err(e) => Err(CardRequestError::UnexpectedCardRequestError(e.to_string())),
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let selected_cards =
                        response
                            .json::<SelectedCardsResponse>()
                            .await
                            .map_err(|_| {
                                return CardRequestError::SelectedCardsParseError;
                            })?;

                    if selected_cards.cards_not_found.len() != 0
                        || selected_cards.invalid_card_guid.len() != 0
                    {
                        let message = format!(
                            "Not found: {}, Invalid cards: {}",
                            selected_cards.cards_not_found.len(),
                            selected_cards.invalid_card_guid.len()
                        );
                        return Err(CardRequestError::MissingCardData(message));
                    }

                    Ok(selected_cards.cards)
                }
                _ => {
                    let response_body = response.text().await.unwrap_or("NO MESSAGE".to_string());
                    Err(CardRequestError::UnexpectedCardRequestError(response_body))
                }
            },
        }
    }
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct CardView {
    pub id: String,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub play_cost: i32,
    
    pub owner_id: String,
    pub effects: Vec<String>,
    pub position: Option<String>,
    
    pub in_deck: bool,
    pub in_hand: bool,
    pub in_board: bool,
    pub in_graveyard: bool,
    pub is_exhausted: bool,
}

impl CardView {
    pub fn create_view(card: &Card, owner_id: String) -> Self {
        CardView {
            position: None,
            owner_id: owner_id,
            is_exhausted: false,
            id: card.id.clone(),
            effects: Vec::new(),
            name: card.name.clone(),
            attack: card.attack.clone(),
            health: card.health.clone(),
            play_cost: card.play_cost.clone(),
            in_deck: false,
            in_hand: false,
            in_board: false,
            in_graveyard: false,
        }
    }
}
