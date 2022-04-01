//! Client upgrade request.
//!
//! From [RFC-6455 Section 4.1](https://datatracker.ietf.org/doc/html/rfc6455#section-4.1):
//!
//! Once a connection to the server has been established (including a
//! connection via a proxy or over a TLS-encrypted tunnel), the client
//! MUST send an opening handshake to the server.  The handshake consists
//! of an HTTP Upgrade request, along with a list of required and
//! optional header fields.
//!
//! Once the client's opening handshake has been sent, the client MUST
//! wait for a response from the server before sending any further data.
//!
//! Example:
//!
//! ```text
//! GET /path HTTP/1.1
//! host: www.example.com
//! upgrade: websocket
//! connection: upgrade
//! sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==
//! sec-websocket-version: 13
//! ```
//!

use super::{HttpHeader, HeaderHelper};
use super::{write_header, filter_header};
use super::handshake_check;
use super::MAX_ALLOW_HEADERS;
use super::{HTTP_METHOD, HTTP_VERSION, HTTP_LINE_BREAK, HTTP_HEADER_SP};
use super::static_headers::*;

use crate::bleed::Writer;
use crate::error::HandshakeError;

/// Http request presentation.
pub struct Request<'h, 'b: 'h, const N: usize = MAX_ALLOW_HEADERS> {
    pub path: &'b [u8],
    pub host: &'b [u8],
    pub sec_key: &'b [u8],
    pub other_headers: &'h mut [HttpHeader<'b>],
}

impl<'h, 'b: 'h, const N: usize> HeaderHelper for Request<'h, 'b, N> {
    const SIZE: usize = N;
}

impl<'h, 'b: 'h> Request<'h, 'b> {
    /// Create with user provided headers, other fields are left empty.
    /// The max decode header size is [`MAX_ALLOW_HEADERS`].
    #[inline]
    pub const fn new(other_headers: &'h mut [HttpHeader<'b>]) -> Self {
        Self {
            path: b"",
            host: b"",
            sec_key: b"",
            other_headers,
        }
    }
}

impl<'h, 'b: 'h, const N: usize> Request<'h, 'b, N> {
    /// Create with user provided headers, other fields are left empty.
    /// The const generic paramater represents the max decode header size.
    #[inline]
    pub const fn new_custom(other_headers: &'h mut [HttpHeader<'b>]) -> Self {
        Self {
            path: b"",
            host: b"",
            sec_key: b"",
            other_headers,
        }
    }

    /// Encode to a provided buffer, return the number of written bytes.
    ///
    /// Necessary headers, including `host`, `upgrade`, `connection`,
    /// `sec-websocket-key` and `sec-websocket-version` are written to
    /// the buffer, then other headers(if any) are written in order.
    ///
    /// Caller should make sure the buffer is large enough,
    /// otherwise a [`HandshakeError::NotEnoughCapacity`] error will be returned.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, HandshakeError> {
        debug_assert!(buf.len() > 80);

        let mut w = Writer::new(buf);

        // GET {path} HTTP/1.1
        unsafe {
            w.write_unchecked(HTTP_METHOD);
            w.write_byte_unchecked(0x20);
            w.write_unchecked(self.path);
            w.write_byte_unchecked(0x20);
            w.write_unchecked(HTTP_VERSION);
            w.write_unchecked(HTTP_LINE_BREAK);
        }

        // host: {host}
        write_header!(w, HEADER_HOST_NAME, self.host);

        // upgrade: websocket
        write_header!(w, HEADER_UPGRADE_NAME, HEADER_UPGRADE_VALUE);

        // connection: upgrade
        write_header!(w, HEADER_CONNECTION_NAME, HEADER_CONNECTION_VALUE);

        // sec-websocket-key: {sec_key}
        write_header!(w, HEADER_SEC_WEBSOCKET_KEY_NAME, self.sec_key);

        // sec-websocket-version: 13
        write_header!(
            w,
            HEADER_SEC_WEBSOCKET_VERSION_NAME,
            HEADER_SEC_WEBSOCKET_VERSION_VALUE
        );

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
    /// Necessary headers, including `host`, `upgrade`, `connection`,
    /// `sec-websocket-key` and `sec-websocket-version` are parsed and checked,
    /// and stored in the struct. Optional headers
    /// (like `sec-websocket-protocol`) are stored in `other_headers`.
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
        let mut request = httparse::Request::new(&mut headers);

        // return value
        let decode_n = match request.parse(buf)? {
            httparse::Status::Complete(n) => n,
            httparse::Status::Partial => return Err(HandshakeError::NotEnoughData),
        };

        // check method
        if request.method.unwrap().as_bytes() != HTTP_METHOD {
            return Err(HandshakeError::HttpMethod);
        }

        // check version, should be HTTP/1.1
        // ref: https://docs.rs/httparse/latest/src/httparse/lib.rs.html#581-596
        if request.version.unwrap() != 1_u8 {
            return Err(HandshakeError::HttpVersion);
        }

        // handle headers below
        // headers are shrunk to number of inited headers
        // ref: https://docs.rs/httparse/latest/src/httparse/lib.rs.html#757-765
        let headers = request.headers;

        let mut required_headers = [
            HEADER_HOST,
            HEADER_UPGRADE,
            HEADER_CONNECTION,
            HEADER_SEC_WEBSOCKET_KEY,
            HEADER_SEC_WEBSOCKET_VERSION,
        ];

        // filter required headers, save other headers
        filter_header(headers, &mut required_headers, self.other_headers);

        let [host_hdr, upgrade_hdr, connection_hdr, sec_key_hdr, sec_version_hdr] =
            required_headers;

        // check missing header
        if !required_headers.iter().all(|h| !h.value.is_empty()) {
            handshake_check!(host_hdr, HandshakeError::HttpHost);
            handshake_check!(upgrade_hdr, HandshakeError::Upgrade);
            handshake_check!(connection_hdr, HandshakeError::Connection);
            handshake_check!(sec_key_hdr, HandshakeError::SecWebSocketKey);
            handshake_check!(sec_version_hdr, HandshakeError::SecWebSocketVersion);
        }

        // check header value (case insensitive)
        // ref: https://datatracker.ietf.org/doc/html/rfc6455#section-4.1
        handshake_check!(upgrade_hdr, HEADER_UPGRADE_VALUE, HandshakeError::Upgrade);

        handshake_check!(
            connection_hdr,
            HEADER_CONNECTION_VALUE,
            HandshakeError::Connection
        );

        handshake_check!(
            sec_version_hdr,
            HEADER_SEC_WEBSOCKET_VERSION_VALUE,
            HandshakeError::SecWebSocketVersion
        );

        // save ref
        self.path = request.path.unwrap().as_bytes();
        self.host = host_hdr.value;
        self.sec_key = sec_key_hdr.value;

        // shrink header reference
        let other_header_len = headers.len() - required_headers.len();

        // remove lifetime here, remember that
        // &mut other_headers lives longer than &mut self
        let other_headers: &'h mut [HttpHeader<'b>] =
            unsafe { &mut *(self.other_headers as *mut _) };
        self.other_headers = unsafe { other_headers.get_unchecked_mut(0..other_header_len) };

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
    fn client_handshake() {
        for i in 0..64 {
            let hdr_len: usize = thread_rng().gen_range(1..128);
            let headers = format!(
                "GET / HTTP/1.1\r\n{}\r\n",
                make_headers(i, hdr_len, TEMPLATE_HEADERS)
            );

            let mut other_headers = HttpHeader::new_custom_storage::<1024>();
            let mut request = Request::<1024>::new_custom(&mut other_headers);
            let decode_n = request.decode(headers.as_bytes()).unwrap();

            assert_eq!(decode_n, headers.len());
            assert_eq!(request.path, b"/");
            assert_eq!(request.host, b"www.example.com");
            assert_eq!(request.sec_key, b"dGhlIHNhbXBsZSBub25jZQ==");

            // other headers
            macro_rules! match_other {
                ($name: expr, $value: expr) => {{
                    request
                        .other_headers
                        .iter()
                        .find(|hdr| hdr.name == $name && hdr.value == $value)
                        .unwrap();
                }};
            }
            match_other!(b"sec-websocket-accept", b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");

            let mut buf: Vec<u8> = vec![0; 0x4000];
            let encode_n = request.encode(&mut buf).unwrap();
            assert_eq!(encode_n, decode_n);
        }
    }

    #[test]
    fn client_handshake2() {
        macro_rules! run {
            ($host: expr, $path: expr, $sec_key: expr) => {{
                let headers = format!(
                    "GET {1} HTTP/1.1\r\n{0}\r\n",
                    make_headers(
                        16,
                        32,
                        &format!(
                            "host: {0}\r\n\
                        sec-websocket-key: {1}\r\n\
                        upgrade: websocket\r\n\
                        connection: upgrade\r\n\
                        sec-websocket-version: 13",
                            $host, $sec_key
                        )
                    ),
                    $path
                );

                let mut other_headers = HttpHeader::new_storage();
                let mut request = Request::new(&mut other_headers);
                let decode_n = request.decode(headers.as_bytes()).unwrap();
                assert_eq!(decode_n, headers.len());
                assert_eq!(request.host, $host.as_bytes());
                assert_eq!(request.path, $path.as_bytes());
                assert_eq!(request.sec_key, $sec_key.as_bytes());

                let mut buf: Vec<u8> = vec![0; 0x4000];
                let encode_n = request.encode(&mut buf).unwrap();
                assert_eq!(encode_n, decode_n);
            }};
        }

        run!("host", "/path", "key");
        run!("www.abc.com", "/path/to", "xxxxxx");
        run!("wwww.www.ww.w", "/path/to/to/path", "xxxxxxyyyy");
    }

    // catch errors ...
}
