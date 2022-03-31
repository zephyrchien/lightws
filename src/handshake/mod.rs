//! Websocket handshake.

pub mod key;
pub mod request;
pub mod response;

pub use request::Request;
pub use response::Response;
pub use key::{new_sec_key, derive_accept_key};

/// 32
pub const MAX_ALLOW_HEADERS: usize = 32;

/// Empty header with dummy reference
pub const EMPTY_HEADER: HttpHeader = HttpHeader::new(b"", b"");

/// 258EAFA5-E914-47DA-95CA-C5AB0DC85B11
pub const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// GET
pub const HTTP_METHOD: &[u8] = b"GET";

/// HTTP/1.1
pub const HTTP_VERSION: &[u8] = b"HTTP/1.1";

/// CRLF
pub const HTTP_LINE_BREAK: &[u8] = b"\r\n";

/// A colon + one SP is prefered
pub const HTTP_HEADER_SP: &[u8] = b": ";

/// HTTP/1.1 101 Switching Protocols
pub const HTTP_STATUS_LINE: &[u8] = b"HTTP/1.1 101 Switching Protocols";

/// Http header, take two references
#[allow(clippy::len_without_is_empty)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HttpHeader<'h> {
    pub name: &'h [u8],
    pub value: &'h [u8],
}

// compile time computation
trait HeaderHelper {
    const SIZE: usize;
}

impl<'h> HttpHeader<'h> {
    /// Constructor, take provided name and value.
    #[inline]
    pub const fn new(name: &'h [u8], value: &'h [u8]) -> Self { Self { name, value } }

    /// Total number of bytes(name + value + sp).
    #[inline]
    pub const fn len(&self) -> usize {
        self.name.len() + self.value.len() + HTTP_HEADER_SP.len() + HTTP_LINE_BREAK.len()
    }

    /// Create [`MAX_ALLOW_HEADERS`] empty headers.
    #[inline]
    pub const fn new_storage() -> [HttpHeader<'static>; MAX_ALLOW_HEADERS] {
        [EMPTY_HEADER; MAX_ALLOW_HEADERS]
    }

    /// Create N empty headers.
    #[inline]
    pub const fn new_custom_storage<const N: usize>() -> [HttpHeader<'static>; N] {
        [EMPTY_HEADER; N]
    }
}

impl Default for HttpHeader<'static> {
    fn default() -> Self { EMPTY_HEADER }
}

impl<'h> std::fmt::Display for HttpHeader<'h> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::str::from_utf8_unchecked;
        write!(
            f,
            "{}: {}",
            unsafe { from_utf8_unchecked(self.name) },
            unsafe { from_utf8_unchecked(self.value) }
        )
    }
}

macro_rules! header {
    (   $(
            $(#[$docs: meta])*
            ($hdr: ident => $name: expr, $value: expr);
        )+
    ) => {
        $(
            $(#[$docs])*
            pub const $hdr: HttpHeader = HttpHeader::new($name, $value);
        )+
    };
    (   $(
            ($hdr_name: ident => $name: expr);
        )+
    ) => {
        $(
            pub const $hdr_name: &[u8] = $name;
        )+
    };
}

macro_rules! write_header {
    ($w: expr, $hdr: expr) => {
        if $w.remaining() < $hdr.len() {
            return Err(HandshakeError::NotEnoughCapacity);
        } else {
            unsafe {
                $w.write_unchecked($hdr.name);
                $w.write_unchecked(HTTP_HEADER_SP);
                $w.write_unchecked($hdr.value);
                $w.write_unchecked(HTTP_LINE_BREAK);
            }
        }
    };
    ($w: expr, $name: expr, $value: expr) => {
        write_header!($w, HttpHeader::new($name, $value));
    };
}

macro_rules! handshake_check {
    ($hdr: expr, $e: expr) => {
        if $hdr.value.is_empty() {
            return Err($e);
        }
    };
    ($hdr: expr, $value: expr, $e: expr) => {
        // header value here is case insensitive
        // ref: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
        if $hdr.value.is_empty() || !$hdr.value.eq_ignore_ascii_case($value) {
            return Err($e);
        }
    };
}

pub(self) use write_header;
pub(self) use handshake_check;

#[inline]
fn filter_header<'h>(
    all: &[httparse::Header<'h>],
    required: &mut [HttpHeader<'h>],
    other: &mut [HttpHeader<'h>],
) {
    let mut other_iter = other.iter_mut();
    for hdr in all.iter() {
        let name = hdr.name.as_bytes();

        if let Some(h) = required
            .iter_mut()
            .filter(|h| h.value.is_empty())
            .find(|h| h.name.eq_ignore_ascii_case(name))
        {
            h.value = hdr.value;
        } else {
            let other_hdr = other_iter.next().unwrap();
            other_hdr.name = name;
            other_hdr.value = hdr.value;
        }
    }
}

/// Static http headers
#[allow(unused)]
pub mod static_headers {
    use super::HttpHeader;
    // header
    header!(
        /// host: {host}
        (HEADER_HOST => b"host", b"");

        /// upgrade: websocket
        (HEADER_UPGRADE => b"upgrade", b"");

        /// connection: upgrade
        (HEADER_CONNECTION => b"connection", b"");

        /// sec-websocket-key: {key}
        (HEADER_SEC_WEBSOCKET_KEY => b"sec-websocket-key", b"");

        /// sec-websocket-accept: {accept}
        (HEADER_SEC_WEBSOCKET_ACCEPT => b"sec-websocket-accept", b"");

        /// sec-webSocket-version: 13
        (HEADER_SEC_WEBSOCKET_VERSION => b"sec-webSocket-version", b"");
    );

    // header name
    header! {
        (HEADER_HOST_NAME => b"host");

        (HEADER_UPGRADE_NAME => b"upgrade");

        (HEADER_CONNECTION_NAME => b"connection");

        (HEADER_SEC_WEBSOCKET_KEY_NAME => b"sec-websocket-key");

        (HEADER_SEC_WEBSOCKET_ACCEPT_NAME => b"sec-websocket-accept");

        (HEADER_SEC_WEBSOCKET_VERSION_NAME => b"sec-websocket-version");
    }

    // header value
    header! {
        (HEADER_UPGRADE_VALUE => b"websocket");

        (HEADER_CONNECTION_VALUE => b"upgrade");

        (HEADER_SEC_WEBSOCKET_VERSION_VALUE => b"13");
    }
}

#[cfg(test)]
mod test {
    use rand::prelude::*;

    pub const TEMPLATE_HEADERS: &str = "\
        host: www.example.com\r\n\
        upgrade: websocket\r\n\
        connection: upgrade\r\n\
        sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\
        sec-websocket-version: 13";

    pub fn make_headers(count: usize, max_len: usize, headers: &str) -> String {
        fn rand_ascii() -> char {
            let x: u8 = thread_rng().gen_range(1..=4);
            let ch: u8 = match x {
                1 => thread_rng().gen_range(b'0'..=b'9'),
                2 => thread_rng().gen_range(b'A'..=b'Z'),
                3 => thread_rng().gen_range(b'a'..=b'z'),
                4 => b'-',
                _ => unreachable!(),
            };
            ch as char
        }

        fn rand_str(len: usize) -> String {
            let mut s = String::new();
            for _ in 0..len {
                s.push(rand_ascii());
            }
            s
        }

        fn make_header(max_len: usize) -> String {
            let mut s = String::with_capacity(256);
            let name_len: usize = thread_rng().gen_range(1..=max_len);
            let value_len: usize = thread_rng().gen_range(1..=max_len);
            s.push_str(&format!(
                "{}: {}\r\n",
                rand_str(name_len),
                rand_str(value_len)
            ));
            s
        }

        let mut s = Vec::<String>::with_capacity(256);
        for hdr in headers.split("\r\n") {
            s.push(format!("{}\r\n", hdr));
        }
        for _ in 0..count {
            s.push(make_header(max_len));
        }
        s.shuffle(&mut thread_rng());
        s.concat()
    }
}
