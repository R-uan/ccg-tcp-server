use super::deck::Card;

#[derive(Default)]
pub struct Board {
    pub creatures: Vec<Card>,
    pub artifacts: Vec<Card>,
    pub enchantments: Vec<Card>,
}

#[derive(Default)]
pub struct Cemetery {
    pub creatures: Vec<Card>,
    pub artifacts: Vec<Card>,
    pub enchantments: Vec<Card>,
}
