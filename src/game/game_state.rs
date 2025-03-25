pub struct GameState {}

impl GameState {
    pub fn new_game() -> Self {
        Self {}
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let xd = b"placeholder: to do function";
        return xd.to_vec().into_boxed_slice();
    }
}
