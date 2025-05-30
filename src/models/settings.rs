use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(rename = "AUTH_SERVER")]
    pub auth_server: String,
    #[serde(rename = "CARD_SERVER")]
    pub card_server: String,
    #[serde(rename = "DECK_SERVER")]
    pub deck_server: String,
}
