use std::io::{Read, Write, Result};
use std::task::Poll;

use super::detail;
use super::Endpoint;

use crate::role::ClientRole;
use crate::handshake::{HttpHeader, Request, Response};
use crate::handshake::{new_sec_key, derive_accept_key};
use crate::error::HandshakeError;
use crate::stream::Stream;

impl<IO: Read + Write, Role: ClientRole> Endpoint<IO, Role> {
    /// Send websocket upgrade request to IO source, return
    /// the number of bytes transmitted.
    /// Request data are encoded to the provided buffer.
    /// This function will block until all data
    /// are written to IO source or an error occurs.
    pub fn send_request_sync<const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        request: &Request<'_, '_, N>,
    ) -> Result<usize> {
        match detail::send_request(io, buf, request, |io, buf| io.write(buf).into()) {
            Poll::Ready(x) => x,
            Poll::Pending => unreachable!(),
        }
    }

    /// Receive websocket upgrade response from IO source, return
    /// the number of bytes transmitted.
    /// Received data are stored in the provided buffer, and parsed
    /// as [`Response`].
    /// This function will block on reading data, until there is enough
    /// data to parse a response or an error occurs.
    /// 
    /// # Safety
    /// 
    /// Caller must not modify the buffer while `response` is in use,
    /// otherwise it is undefined behavior!
    pub unsafe fn recv_response_sync<'h, 'b: 'h, const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        response: &mut Response<'h, 'b, N>,
    ) -> Result<usize> {
        match detail::recv_response(io, buf, response, |io, buf| io.read(buf).into()) {
            Poll::Ready(x) => x,
            Poll::Pending => unreachable!(),
        }
    }

    /// Perform a simple websocket client handshake, return a new websocket stream.
    /// This function is a combination of [`send_request`](Self::send_request_sync)
    /// and [`recv_response`](Self::recv_response_sync), without accessing [`Response`].
    /// it will block until the handshake completes, or an error occurs.
    pub fn connect_sync(
        mut io: IO,
        buf: &mut [u8],
        host: &str,
        path: &str,
    ) -> Result<Stream<IO, Role>> {
        let sec_key = new_sec_key();
        let sec_accept = derive_accept_key(&sec_key);

        // send
        let request = Request::new(path.as_bytes(), host.as_bytes(), &sec_key);
        let _ = Self::send_request_sync(&mut io, buf, &request)?;

        // recv
        let mut other_headers = HttpHeader::new_storage();
        let mut response = Response::new_storage(&mut other_headers);
        // this is safe since we do not modify response.
        let _ = unsafe { Self::recv_response_sync(&mut io, buf, &mut response) }?;

        // check
        if response.sec_accept != sec_accept {
            return Err(HandshakeError::SecWebSocketAccept.into());
        }

        Ok(Stream::new(io))
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use super::*;
    use super::super::test::*;
    use crate::error::HandshakeError;
    use crate::role::Client;

    #[test]
    fn send_upgrade_request() {
        fn run_limit(limit: usize) {
            let mut rw = LimitReadWriter {
                rbuf: Vec::new(),
                wbuf: Vec::new(),
                rlimit: 0,
                wlimit: limit,
                cursor: 0,
            };

            let request = Request::new(b"/ws", b"www.example.com", b"dGhlIHNhbXBsZSBub25jZQ==");

            let mut buf = vec![0u8; 1024];

            let send_n =
                Endpoint::<_, Client>::send_request_sync(&mut rw, &mut buf, &request).unwrap();

            assert_eq!(send_n, REQUEST.len());
            assert_eq!(&buf[..send_n], REQUEST);
        }

        for i in 1..=256 {
            run_limit(i);
        }
    }

    #[test]
    fn recv_upgrade_response() {
        fn run_limit(limit: usize) {
            let mut rw = LimitReadWriter {
                rbuf: Vec::from(RESPONSE),
                wbuf: Vec::new(),
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = vec![0u8; 1024];
            let mut headers = HttpHeader::new_storage();
            let mut response = Response::new_storage(&mut headers);

            let recv_n = unsafe {
                Endpoint::<_, Client>::recv_response_sync(&mut rw, &mut buf, &mut response)
            }
            .unwrap();

            assert_eq!(recv_n, RESPONSE.len());
            assert_eq!(response.sec_accept, b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
            drop(response);
            assert_eq!(&buf[..recv_n], RESPONSE);
        }

        for i in 1..=256 {
            run_limit(i);
        }
    }

    #[test]
    fn client_connect() {
        // use std::error::Error;
        let mut rw = LimitReadWriter {
            rbuf: Vec::from(RESPONSE),
            wbuf: Vec::new(),
            rlimit: 1,
            wlimit: 1,
            cursor: 0,
        };

        let mut buf = vec![0u8; 1024];

        // sec-websocket-accept mismatch
        // since connect uses a random key
        let stream = Endpoint::<_, Client>::connect_sync(&mut rw, &mut buf, "example.com", "/");
        if let Err(e) = stream {
            let e = e.source().unwrap();
            let e: &HandshakeError = e.downcast_ref().unwrap();
            assert_eq!(*e, HandshakeError::SecWebSocketAccept);
        }
    }
}
