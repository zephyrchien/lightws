#![allow(missing_docs)]
//! Errors

mod frame;
mod handshake;

pub use frame::FrameError;
pub use handshake::HandshakeError;

use std::fmt::{Display, Formatter};
use std::convert::Infallible;

#[derive(Debug)]
pub enum Error {
    Frame(FrameError),

    Handshake(HandshakeError),
}

impl From<FrameError> for Error {
    fn from(e: FrameError) -> Self { Error::Frame(e) }
}

impl From<HandshakeError> for Error {
    fn from(e: HandshakeError) -> Self { Error::Handshake(e) }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Frame(e) => write!(f, "Frame error: {}", e),
            Handshake(e) => write!(f, "Handshake error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use Error::*;

        match self {
            Frame(e) => Some(e),
            Handshake(e) => Some(e),
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        use std::io::{Error, ErrorKind};
        Error::new(ErrorKind::Other, e)
    }
}

impl From<Error> for Result<Infallible, std::io::Error> {
    fn from(e: Error) -> Self { Err(e.into()) }
}
