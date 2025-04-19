use super::card::Card;

#[derive(Default)]
pub struct Board {
    pub creatures: Vec<Card>,
    pub artifacts: Vec<Card>,
    pub enchantments: Vec<Card>,
}
