pub struct GameState {}

impl GameState {
    pub fn new_game() -> Self {
        Self {}
    }

    pub fn wrap_game_state(&self) -> Box<[u8]> {
        let xd = b"placeholder: to do gamestate";
        return xd.to_vec().into_boxed_slice();
    }
}
