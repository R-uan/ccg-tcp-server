use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("Invalid player payload")]
    InvalidPlayerPayload,

    #[error("Player token was not authorized")]
    UnauthorizedPlayerError,

    #[error("Invalid deck formatting")]
    InvalidDeckError,

    #[error("Unexpected error")]
    UnexpectedPlayerError,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerConnectionError {
    #[error("Invalid player payload")]
    InvalidPlayerPayload,

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
pub enum ProtocolError {}

#[derive(Debug)]
pub struct InvalidHeaderError;

impl fmt::Display for InvalidHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid protocol header.")
    }
}

#[derive(Debug)]
pub struct PackageWriteError;

impl fmt::Display for PackageWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unable to send package through client stream.")
    }
}

#[derive(Debug)]
pub struct NoAddrError;

impl fmt::Display for NoAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Could not get client addr.")
    }
}

#[derive(Debug, PartialEq)]
pub struct InvalidPlayerPayload;

impl fmt::Display for InvalidPlayerPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "player payload invalid")
    }
}
