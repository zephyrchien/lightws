use std::io::Result;
use std::task::{Poll, ready};

use crate::handshake::Request;
use crate::handshake::Response;
use crate::error::HandshakeError;

pub fn send_response<F, IO, const N: usize>(
    io: &mut IO,
    buf: &mut [u8],
    response: &Response<N>,
    mut write: F,
) -> Poll<Result<usize>>
where
    F: FnMut(&mut IO, &[u8]) -> Poll<Result<usize>>,
{
    let total = match response.encode(buf) {
        Ok(n) => n,
        Err(e) => return Poll::Ready(Err(e.into())),
    };

    let mut offset = 0;

    while offset < total {
        let n = ready!(write(io, &buf[offset..total]))?;

        offset += n;
    }

    Poll::Ready(Ok(total))
}

pub fn recv_request<'h, 'b: 'h, F, IO, const N: usize>(
    io: &mut IO,
    buf: &'b mut [u8],
    request: &mut Request<'h, 'b, N>,
    mut read: F,
) -> Poll<Result<usize>>
where
    F: FnMut(&mut IO, &mut [u8]) -> Poll<Result<usize>>,
{
    let total = buf.len();
    let mut offset = 0;

    // WARNING !! I am breaking rust's borrow rules here.
    // Caller must not modify the buffer while response is in use.
    let buf_const: &'b [u8] = unsafe { &*(buf as *const [u8]) };

    while offset < total {
        let n = ready!(read(io, &mut buf[offset..]))?;

        // EOF, no more data
        if n == 0 {
            return Poll::Ready(Err(HandshakeError::NotEnoughData.into()));
        }

        offset += n;

        match request.decode(&buf_const[..offset]) {
            Ok(_) => return Poll::Ready(Ok(offset)),
            Err(ref e) if *e == HandshakeError::NotEnoughData => continue,
            Err(e) => return Poll::Ready(Err(e.into())),
        }
    }

    // provided buffer is filled, however it could not accommodate the response.
    Poll::Ready(Err(HandshakeError::NotEnoughCapacity.into()))
}
