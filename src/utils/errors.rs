#[derive(Debug, thiserror::Error)]
pub enum PlayerConnectionError {
    #[error("{0}")]
    InvalidResponseBody(String),

    #[error("`{0}` is banned")]
    BannedPlayer(String),

    #[error("Player connection error: {0}")]
    InvalidPlayerPayload(String),

    #[error("Player's identification does not match with profile")]
    PlayerDiscrepancy,

    #[error("Player is not connected to the match")]
    PlayerNotConnected,

    #[error("Player token was not authorized")]
    UnauthorizedPlayerError,

    #[error("Unexpected player error: {0}")]
    UnexpectedPlayerError(String),

    #[error("Deck was not found")]
    DeckNotFound,

    #[error("Deck format invalid")]
    InvalidDeckFormat,

    #[error("Deck error: {0}")]
    UnexpectedDeckError(String),

    #[error("Player does not have permission to access deck")]
    UnauthorizedDeckError,

    #[error("{0}")]
    InternalError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Could not successfully parse protocol header: {0}")]
    InvalidHeaderError(String),

    #[error("Invalid packet: {0}")]
    InvalidPacketError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Could not send package: {0}")]
    PackageWriteError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GameLogicError {
    #[error("Card played is not in hand")]
    CardPlayedIsNotInHand,

    #[error("Unable to get card details")]
    UnableToGetCardDetails,

    #[error("Player ID does not match with request's")]
    PlayerIdDoesNotMatch,

    #[error("Player was not found in Hashmap")]
    PlayerNotFound,
    
    #[error("Function `{0}` was not found for card `{1}`")]
    FunctionNotFound(String, String),

    #[error("Unable to call Lua function `{0}`")]
    FunctionNotCallable(String),

    #[error("Invalid GameAction return")]
    InvalidGameActions,

    #[error("Not player's turn")]
    NotPlayerTurn,
}

#[derive(Debug, thiserror::Error)]
pub enum CardRequestError {
    #[error("Card not found: `{0}`")]
    CardNotFound(String),
    
    #[error("Unexpected card error: {0}")]
    UnexpectedCardRequestError(String),

    #[error("Failed to get full cards data from API")]
    FailedToGetFullCardsData,
    
    #[error("Failed to get cards data: {0}")]
    MissingCardData(String),

    #[error("Failed to parse full cards response")]
    SelectedCardsParseError
}

#[derive(Debug, thiserror::Error)]
pub enum GameInstanceError {
    #[error("Placeholder error, make a specific one")]
    PlaceHolderError
}

#[derive(Debug, thiserror::Error)]
pub enum ServerInstanceError {
    #[error("Placeholder error, make a specific one")]
    PlaceHolderError,
    
    #[error("Placeholder error, make a specific one")]
    AlreadyInitialized,
    
    #[error("Failed to create Game Instance: {0}")]
    GameInstanceFail(String),
    
    #[error("Unable to unwrap UninitializedServer")]
    UnwrapFailed
}
