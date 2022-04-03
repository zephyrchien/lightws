use std::io::Result;
use std::pin::Pin;
use std::future::poll_fn;

use tokio::io::{ReadBuf, AsyncRead, AsyncWrite};

use super::detail;
use super::Endpoint;

use crate::role::ClientRole;
use crate::handshake::{HttpHeader, Request, Response};
use crate::handshake::{new_sec_key, derive_accept_key};
use crate::error::HandshakeError;
use crate::stream::Stream;

impl<IO: AsyncRead + AsyncWrite + Unpin, Role: ClientRole> Endpoint<IO, Role> {
    /// Async version of [`send_request`](Self::send_request).
    pub async fn send_request_async<'h, 'b: 'h, const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        request: &Request<'h, 'b, N>,
    ) -> Result<usize> {
        poll_fn(|cx| {
            detail::send_request(io, buf, request, |io, buf| Pin::new(io).poll_write(cx, buf))
        })
        .await
    }

    /// Async version of [`recv_response`](Self::recv_response).
    ///
    /// # Safety
    ///
    /// Caller must not modify the buffer while `response` is in use,
    /// otherwise it is undefined behavior!
    pub async unsafe fn recv_response_async<'h, 'b: 'h, const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        response: &mut Response<'h, 'b, N>,
    ) -> Result<usize> {
        poll_fn(|cx| {
            detail::recv_response(io, buf, response, |io, buf| {
                let mut buf = ReadBuf::new(buf);
                Pin::new(io)
                    .poll_read(cx, &mut buf)
                    .map_ok(|_| buf.filled().len())
            })
        })
        .await
    }

    /// Async version of [`connect`](Self::connect).
    pub async fn connect_async(
        mut io: IO,
        buf: &mut [u8],
        host: &str,
        path: &str,
    ) -> Result<Stream<IO, Role>> {
        let sec_key = new_sec_key();
        let sec_accept = derive_accept_key(&sec_key);

        // send
        let request = Request::new(path.as_bytes(), host.as_bytes(), &sec_key);
        let _ = Self::send_request_async(&mut io, buf, &request).await?;

        // recv
        let mut other_headers = HttpHeader::new_storage();
        let mut response = Response::new_storage(&mut other_headers);
        // this is safe since we do not modify response.
        let _ = unsafe { Self::recv_response_async(&mut io, buf, &mut response) }.await?;

        // check
        if response.sec_accept != sec_accept {
            return Err(HandshakeError::SecWebSocketAccept.into());
        }

        Ok(Stream::new(io))
    }
}
