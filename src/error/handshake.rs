use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum HandshakeError {
    // http error
    HttpVersion,

    HttpMethod,

    HttpSatusCode,

    HttpHost,

    // websocket error
    Upgrade,

    Connection,

    SecWebSocketKey,

    SecWebSocketAccept,

    SecWebSocketVersion,

    // other error

    // read
    NotEnoughData,

    // write
    NotEnoughCapacity,

    Httparse(httparse::Error),
}

impl Display for HandshakeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use HandshakeError::*;
        match self {
            // http error
            HttpVersion => write!(f, "Illegal http version"),

            HttpMethod => write!(f, "Illegal http method"),

            HttpSatusCode => write!(f, "Illegal http status code"),

            HttpHost => write!(f, "Missing http host header"),

            // websocket error
            Upgrade => write!(f, "Missing or illegal upgrade header"),

            Connection => write!(f, "Missing or illegal connection header"),

            SecWebSocketKey => {
                write!(f, "Missing sec-websocket-key header")
            }

            SecWebSocketAccept => {
                write!(f, "Missing or illegal sec-websocket-accept header")
            }

            SecWebSocketVersion => {
                write!(f, "Missing or illegal sec-websocket-version")
            }

            // other error
            NotEnoughData => write!(f, "Not enough data to parse"),

            NotEnoughCapacity => write!(f, "Not enough space to write to"),

            Httparse(e) => write!(f, "Http parse error: {}", e),
        }
    }
}

impl From<httparse::Error> for HandshakeError {
    fn from(e: httparse::Error) -> Self { HandshakeError::Httparse(e) }
}

impl std::error::Error for HandshakeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let HandshakeError::Httparse(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
