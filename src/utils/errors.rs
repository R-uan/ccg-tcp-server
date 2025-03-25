use std::fmt;

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
