#![allow(missing_docs)]
//! Errors

mod frame;
mod handshake;

pub use frame::FrameError;
pub use handshake::HandshakeError;

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Frame(FrameError),

    Handshake(HandshakeError),

    Io(std::io::Error),
}

impl From<FrameError> for Error {
    fn from(e: FrameError) -> Self { Error::Frame(e) }
}

impl From<HandshakeError> for Error {
    fn from(e: HandshakeError) -> Self { Error::Handshake(e) }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error { Error::Io(e) }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Frame(e) => write!(f, "Frame error: {}", e),
            Handshake(e) => write!(f, "Handshake error: {}", e),
            Io(e) => write!(f, "Io error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use Error::*;

        match self {
            Frame(e) => e.source(),
            Handshake(e) => e.source(),
            Io(e) => e.source(),
        }
    }
}
