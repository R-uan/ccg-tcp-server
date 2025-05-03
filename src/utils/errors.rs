#[derive(Debug, thiserror::Error)]
pub enum PlayerConnectionError {
    #[error("Invalid player payload: {0}")]
    InvalidPlayerPayload(String),

    #[error("Player token was not authorized")]
    UnauthorizedPlayerError,

    #[error("Unexpected error")]
    UnexpectedPlayerError,

    #[error("Deck was not found")]
    DeckNotFound,

    #[error("Deck format invalid")]
    InvalidDeckFormat,

    #[error("Unexpected deck error")]
    UnexpectedDeckError,

    #[error("Player does not have permission to access deck")]
    UnauthorizedDeckError,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Could not successfuly parse protocol header: {0}")]
    InvalidHeaderError(String),

    #[error("Invalid packet: {0}")]
    InvalidPacketError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Could not send package: {0}")]
    PackageWriteError(String),
}
