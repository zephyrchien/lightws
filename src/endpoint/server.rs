use std::io::{Read, Write, Result};

use super::detail;
use super::Endpoint;

use crate::role::ServerRole;
use crate::handshake::{HttpHeader, Request, Response};
use crate::handshake::derive_accept_key;
use crate::error::{Error, HandshakeError};
use crate::stream::Stream;

impl<IO: Read + Write, Role: ServerRole> Endpoint<IO, Role> {
    /// Send websocket upgrade response to IO source, return
    /// the number of bytes transferred.
    /// Response data are encoded to the provided buffer.
    /// This function will block until all data
    /// are written to IO source or an error occurs.
    pub fn send_response(io: &mut IO, buf: &mut [u8], response: &Response) -> Result<usize> {
        detail::send_response(io, buf, response, |io, buf| io.write(buf))
    }

    /// Receive websocket upgrade request from IO source, return
    /// the number of bytes transferred.
    /// Received data are stored in the provided buffer, and parsed
    /// as [`Request`]. **Caller must not modify the buffer while
    /// `request` is in use, otherwise it is undefined behavior!!**
    /// This function will block on reading data, until there is enough
    /// data to parse a request or an error occurs.
    pub fn recv_request<'h, 'b: 'h>(
        io: &mut IO,
        buf: &'b mut [u8],
        request: &mut Request<'h, 'b>,
    ) -> Result<usize> {
        detail::recv_request(io, buf, request, |io, buf| io.read(buf))
    }

    /// Perform a simple websocket server handshake, return a new websocket stream.
    /// This function is a combination of [`recv_request`](Self::recv_request)
    /// and [`send_response`](Self::send_response), without accessing [`Request`].
    /// it will block until the handshake completes, or an error occurs.    
    pub fn accept(mut io: IO, buf: &mut [u8], host: &str, path: &str) -> Result<Stream<IO, Role>> {
        // recv
        let mut other_headers = HttpHeader::new_storage();
        let mut request = Request::new(&mut other_headers);
        let _ = Self::recv_request(&mut io, buf, &mut request)?;

        // check
        if request.host != host.as_bytes() {
            return Err(Error::Handshake(HandshakeError::Manual("host mismatch")).into());
        }

        if request.path != path.as_bytes() {
            return Err(Error::Handshake(HandshakeError::Manual("path mismatch")).into());
        }

        // send
        let sec_accept = derive_accept_key(request.sec_key);
        let response = Response {
            sec_accept: &sec_accept,
            other_headers: &mut [],
        };
        let _ = Self::send_response(&mut io, buf, &response)?;

        Ok(Stream::new(io))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::test::*;
    use crate::role::Server;

    #[test]
    fn send_upgrade_response() {
        fn run_limit(limit: usize) {
            let mut rw = LimitReadWriter {
                rbuf: Vec::new(),
                wbuf: Vec::new(),
                rlimit: 0,
                wlimit: limit,
                cursor: 0,
            };

            let response = Response {
                sec_accept: b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
                other_headers: &mut [],
            };

            let mut buf = vec![0u8; 1024];

            let send_n =
                Endpoint::<_, Server>::send_response(&mut rw, &mut buf, &response).unwrap();

            assert_eq!(send_n, RESPONSE.len());
            assert_eq!(&buf[..send_n], RESPONSE);
        }

        for i in 1..=256 {
            run_limit(i);
        }
    }

    #[test]
    fn recv_upgrade_request() {
        fn run_limit(limit: usize) {
            let mut rw = LimitReadWriter {
                rbuf: Vec::from(REQUEST),
                wbuf: Vec::new(),
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = vec![0u8; 1024];
            let mut headers = HttpHeader::new_storage();
            let mut request = Request::new(&mut headers);

            let recv_n =
                Endpoint::<_, Server>::recv_request(&mut rw, &mut buf, &mut request).unwrap();

            assert_eq!(recv_n, REQUEST.len());
            assert_eq!(request.host, b"www.example.com");
            assert_eq!(request.path, b"/ws");
            assert_eq!(request.sec_key, b"dGhlIHNhbXBsZSBub25jZQ==");
            drop(request);
            assert_eq!(&buf[..recv_n], REQUEST);
        }

        for i in 1..=256 {
            run_limit(i);
        }
    }

    #[test]
    fn server_accept() {
        // use std::error::Error;
        let mut rw = LimitReadWriter {
            rbuf: Vec::from(REQUEST),
            wbuf: Vec::new(),
            rlimit: 1,
            wlimit: 1,
            cursor: 0,
        };

        let mut buf = vec![0u8; 1024];

        let stream = Endpoint::<_, Server>::accept(&mut rw, &mut buf, "www.example.com", "/ws");

        println!("{:?}", stream);
    }
}
