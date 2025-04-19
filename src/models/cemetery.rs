use super::card::Card;

#[derive(Default)]
pub struct Cemetery {
    pub creatures: Vec<Card>,
    pub artifacts: Vec<Card>,
    pub enchantments: Vec<Card>,
}
