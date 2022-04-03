use std::io::Result;
use std::pin::Pin;
use std::future::poll_fn;

use tokio::io::{ReadBuf, AsyncRead, AsyncWrite};

use super::detail;
use super::Endpoint;

use crate::role::ServerRole;
use crate::handshake::{HttpHeader, Request, Response};
use crate::handshake::derive_accept_key;
use crate::error::HandshakeError;
use crate::stream::Stream;

impl<IO: AsyncRead + AsyncWrite + Unpin, Role: ServerRole> Endpoint<IO, Role> {
    /// Async version of [`send_response`](Self::send_response_sync).
    pub async fn send_response<const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        response: &Response<'_, '_, N>,
    ) -> Result<usize> {
        poll_fn(|cx| {
            detail::send_response(io, buf, response, |io, buf| {
                Pin::new(io).poll_write(cx, buf)
            })
        })
        .await
    }

    /// Async version of [`recv_request`](Self::recv_request_sync).
    /// 
    /// # Safety
    /// 
    /// Caller must not modify the buffer while `request` is in use,
    /// otherwise it is undefined behavior!
    pub async unsafe fn recv_request<'h, 'b: 'h, const N: usize>(
        io: &mut IO,
        buf: &mut [u8],
        request: &mut Request<'h, 'b, N>,
    ) -> Result<usize> {
        poll_fn(|cx| {
            detail::recv_request(io, buf, request, |io, buf| {
                let mut buf = ReadBuf::new(buf);
                Pin::new(io)
                    .poll_read(cx, &mut buf)
                    .map_ok(|_| buf.filled().len())
            })
        })
        .await
    }

    /// Async version of [`accept`](Self::accept_sync).
    pub async fn accept(
        mut io: IO,
        buf: &mut [u8],
        host: &str,
        path: &str,
    ) -> Result<Stream<IO, Role>> {
        // recv
        let mut other_headers = HttpHeader::new_storage();
        let mut request = Request::new_storage(&mut other_headers);
        // this is safe since we do not modify request.
        let _ = unsafe { Self::recv_request(&mut io, buf, &mut request) }.await?;

        // check
        if request.host != host.as_bytes() {
            return Err(HandshakeError::Manual("host mismatch").into());
        }

        if request.path != path.as_bytes() {
            return Err(HandshakeError::Manual("path mismatch").into());
        }

        // send
        let sec_accept = derive_accept_key(request.sec_key);
        let response = Response::new(&sec_accept);
        let _ = Self::send_response(&mut io, buf, &response).await?;

        Ok(Stream::new(io))
    }
}
