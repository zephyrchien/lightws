//! Server handshake.
//!
//! From [RFC-6455 Section 4.2](https://datatracker.ietf.org/doc/html/rfc6455#section-4.2):
//!
//! When a client starts a WebSocket connection, it sends its part of the
//! opening handshake.  The server must parse at least part of this
//! handshake in order to obtain the necessary information to generate
//! the server part of the handshake.
//!
//! If the server chooses to accept the incoming connection, it MUST
//! reply with a valid HTTP response.
//!
//! Example:
//!
//! ```text
//! HTTP/1.1 101 Switching Protocols
//! upgrade: websocket
//! connection: upgrade
//! sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
//! ```
//!

use super::{HttpHeader, HeaderHelper};
use super::{write_header, filter_header};
use super::handshake_check;
use super::MAX_ALLOW_HEADERS;
use super::{HTTP_STATUS_LINE, HTTP_LINE_BREAK, HTTP_HEADER_SP};
use super::static_headers::*;

use crate::bleed::Writer;
use crate::error::HandshakeError;

/// Http response presentation.
pub struct Response<'h, 'b: 'h, const N: usize = MAX_ALLOW_HEADERS> {
    pub sec_accept: &'b [u8],
    pub other_headers: &'h mut [HttpHeader<'b>],
}

impl<'h, 'b: 'h, const N: usize> HeaderHelper for Response<'h, 'b, N> {
    const SIZE: usize = N;
}

impl<'h, 'b: 'h> Response<'h, 'b> {
    /// Create with user provided headers, other fields are left empty.
    /// The max decode header size is [`MAX_ALLOW_HEADERS`].
    #[inline]
    pub const fn new(other_headers: &'h mut [HttpHeader<'b>]) -> Self {
        Self {
            sec_accept: b"",
            other_headers,
        }
    }
}

impl<'h, 'b: 'h, const N: usize> Response<'h, 'b, N> {
    /// Create with user provided headers, other fields are left empty.
    /// The const generic paramater represents the max decode header size.
    #[inline]
    pub const fn new_custom(other_headers: &'h mut [HttpHeader<'b>]) -> Self {
        Self {
            sec_accept: b"",
            other_headers,
        }
    }

    /// Encode to a provided buffer, return the number of written bytes.
    ///
    /// Necessary headers, including `upgrade`, `connection`, and
    /// `sec-websocket-accept` are written to the buffer,
    /// then other headers(if any) are written in order.
    ///
    /// Caller should make sure the buffer is large enough,
    /// otherwise a [`HandshakeError::NotEnoughCapacity`] error will be returned.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, HandshakeError> {
        debug_assert!(buf.len() > 80);

        let mut w = Writer::new(buf);

        // HTTP/1.1 101 Switching Protocols
        unsafe {
            w.write_unchecked(HTTP_STATUS_LINE);
            w.write_unchecked(HTTP_LINE_BREAK);
        }

        // upgrade: websocket
        write_header!(w, HEADER_UPGRADE_NAME, HEADER_UPGRADE_VALUE);

        // connection: upgrade
        write_header!(w, HEADER_CONNECTION_NAME, HEADER_CONNECTION_VALUE);

        // sec-websocket-accept: {sec_accept}
        write_header!(w, HEADER_SEC_WEBSOCKET_ACCEPT_NAME, self.sec_accept);

        // other headers
        for hdr in self.other_headers.iter() {
            write_header!(w, hdr)
        }

        // finish with CRLF
        w.write_or_err(HTTP_LINE_BREAK, || HandshakeError::NotEnoughCapacity)?;

        Ok(w.pos())
    }

    /// Parse from a provided buffer, save the results, and
    /// return the number of bytes parsed.
    ///
    /// Necessary headers, including `upgrade`, `connection`, and
    /// `sec-websocket-version` are parsed and checked,
    /// and stored in the struct. Optional headers
    /// (like `sec-websocket-protocol`) are stored in `other headers`.
    /// After the parse, `other_headers` will be shrunk to
    /// fit the number of stored headers.
    ///
    /// Caller should make sure there is enough space
    /// (default is [`MAX_ALLOW_HEADERS`]) to store headers,
    /// which could be specified by the const generic paramater.
    /// If the buffer does not contain a complete http request,
    /// a [`HandshakeError::NotEnoughData`] error will be returned.
    /// If the required headers(mentioned above) do not pass the check
    /// (case insensitive), other corresponding errors will be returned.
    pub fn decode(&mut self, buf: &'b [u8]) -> Result<usize, HandshakeError> {
        debug_assert!(self.other_headers.len() >= <Self as HeaderHelper>::SIZE);

        let mut headers = [httparse::EMPTY_HEADER; N];
        let mut response = httparse::Response::new(&mut headers);

        // return value
        let decode_n = match response.parse(buf)? {
            httparse::Status::Complete(n) => n,
            httparse::Status::Partial => {
                return Err(HandshakeError::NotEnoughData)
            }
        };

        // check version, should be HTTP/1.1
        // ref: https://docs.rs/httparse/latest/src/httparse/lib.rs.html#581-596
        if response.version.unwrap() != 1_u8 {
            return Err(HandshakeError::HttpVersion);
        }

        // check status code, should be 101
        // ref: https://docs.rs/httparse/latest/src/httparse/lib.rs.html#581-596
        if response.code.unwrap() != 101_u16 {
            return Err(HandshakeError::HttpSatusCode);
        }

        // handle headers below
        // headers are shrunk to number of inited headers
        // ref: https://docs.rs/httparse/latest/src/httparse/lib.rs.html#757-765
        let headers = response.headers;

        let mut required_headers = [
            HEADER_UPGRADE,
            HEADER_CONNECTION,
            HEADER_SEC_WEBSOCKET_ACCEPT,
        ];

        // filter required headers, save other headers
        filter_header(headers, &mut required_headers, self.other_headers);

        let [upgrade_hdr, connection_hdr, sec_accept_hdr] = required_headers;

        // check missing header
        if !required_headers.iter().all(|h| !h.value.is_empty()) {
            handshake_check!(upgrade_hdr, HandshakeError::Upgrade);
            handshake_check!(connection_hdr, HandshakeError::Connection);
            handshake_check!(
                sec_accept_hdr,
                HandshakeError::SecWebSocketAccept
            );
        }

        // check header value (case insensitive)
        // ref: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
        handshake_check!(
            upgrade_hdr,
            HEADER_UPGRADE_VALUE,
            HandshakeError::Upgrade
        );

        handshake_check!(
            connection_hdr,
            HEADER_CONNECTION_VALUE,
            HandshakeError::Connection
        );

        // save ref
        self.sec_accept = sec_accept_hdr.value;

        // shrink header reference
        let other_header_len = headers.len() - required_headers.len();

        // remove lifetime here, this does not affect that
        // &mut other_headers live longer than &mut self
        let other_headers: &'h mut [HttpHeader<'b>] =
            unsafe { &mut *(self.other_headers as *mut _) };
        self.other_headers =
            unsafe { other_headers.get_unchecked_mut(0..other_header_len) };

        Ok(decode_n)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::HttpHeader;
    use super::super::test::{make_headers, TEMPLATE_HEADERS};
    use rand::prelude::*;

    #[test]
    fn server_handshake() {
        for i in 0..64 {
            let hdr_len: usize = thread_rng().gen_range(1..128);
            let headers = format!(
                "HTTP/1.1 101 Switching Protocols\r\n{}\r\n",
                make_headers(i, hdr_len, TEMPLATE_HEADERS)
            );

            let mut other_headers = HttpHeader::new_custom_storage::<1024>();
            let mut response = Response::<1024>::new_custom(&mut other_headers);
            let decode_n = response.decode(headers.as_bytes()).unwrap();

            assert_eq!(decode_n, headers.len());
            assert_eq!(response.sec_accept, b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");

            // other headers
            macro_rules! match_other {
                ($name: expr, $value: expr) => {{
                    response
                        .other_headers
                        .iter()
                        .find(|hdr| hdr.name == $name && hdr.value == $value)
                        .unwrap();
                }};
            }

            match_other!(b"host", b"www.example.com");
            match_other!(b"sec-websocket-version", b"13");
            match_other!(b"sec-websocket-key", b"dGhlIHNhbXBsZSBub25jZQ==");

            let mut buf: Vec<u8> = vec![0; 0x4000];
            let encode_n = response.encode(&mut buf).unwrap();
            assert_eq!(encode_n, decode_n);
        }
    }

    #[test]
    fn server_handshake2() {
        macro_rules! run {
            ($sec_accept: expr) => {{
                let headers = format!(
                    "HTTP/1.1 101 Switching Protocols\r\n{}\r\n",
                    make_headers(
                        16,
                        32,
                        &format!(
                            "upgrade: websocket\r\n\
                        connection: upgrade\r\n\
                        sec-websocket-accept: {}",
                            $sec_accept
                        )
                    )
                );

                let mut other_headers = HttpHeader::new_storage();
                let mut response = Response::new(&mut other_headers);
                let decode_n = response.decode(headers.as_bytes()).unwrap();
                assert_eq!(decode_n, headers.len());
                assert_eq!(response.sec_accept, $sec_accept.as_bytes());

                let mut buf: Vec<u8> = vec![0; 0x4000];
                let encode_n = response.encode(&mut buf).unwrap();
                assert_eq!(encode_n, decode_n);
            }};
        }

        run!("aaa");
        run!("bbbbbbbbbb");
        run!("xxxxxxxxx==");
    }

    // catch errors ...
}
