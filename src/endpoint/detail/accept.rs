use std::ops::Try;

use crate::handshake::Request;
use crate::handshake::Response;
use crate::error::{Error, HandshakeError};

pub fn send_response<F, T, E, IO, const N: usize>(
    io: &mut IO,
    buf: &mut [u8],
    response: &Response<N>,
    mut write: F,
) -> T
where
    F: FnMut(&mut IO, &[u8]) -> T,
    T: Try<Output = usize, Residual = E>,
    E: From<Error>,
{
    let total = match response.encode(buf) {
        Ok(n) => n,
        Err(e) => return T::from_residual(Error::from(e).into()),
    };

    let mut offset = 0;

    while offset < total {
        let n = write(io, &buf[offset..total])?;

        offset += n;
    }

    T::from_output(total)
}

pub fn recv_request<'h, 'b: 'h, F, T, E, IO, const N: usize>(
    io: &mut IO,
    buf: &'b mut [u8],
    request: &mut Request<'h, 'b, N>,
    mut read: F,
) -> T
where
    F: FnMut(&mut IO, &mut [u8]) -> T,
    T: Try<Output = usize, Residual = E>,
    E: From<Error>,
{
    let total = buf.len();
    let mut offset = 0;

    // WARNING !! I am breaking rust's borrow rules here.
    // Caller must not modify the buffer while response is in use.
    let buf_const: &'b [u8] = unsafe { &*(buf as *const [u8]) };

    while offset < total {
        let n = read(io, &mut buf[offset..])?;

        // EOF, no more data
        if n == 0 {
            return T::from_residual(Error::from(HandshakeError::NotEnoughData).into());
        }

        offset += n;

        match request.decode(&buf_const[..offset]) {
            Ok(_) => return T::from_output(offset),
            Err(ref e) if *e == HandshakeError::NotEnoughData => continue,
            Err(e) => return T::from_residual(Error::from(e).into()),
        }
    }

    // provided buffer is filled, however it could not accommodate the response.
    T::from_residual(Error::from(HandshakeError::NotEnoughCapacity).into())
}
