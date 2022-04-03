//! Websocket endpoint.

mod detail;
mod client;
mod server;

cfg_if::cfg_if! {
    if #[cfg(feature = "tokio")] {
        mod async_client;
        mod async_server;
    }
}

use std::marker::PhantomData;

/// Client or server endpoint.
pub struct Endpoint<IO, Role> {
    _marker: PhantomData<IO>,
    __marker: PhantomData<Role>,
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write, Result};

    pub const REQUEST: &[u8] = b"\
    GET /ws HTTP/1.1\r\n\
    host: www.example.com\r\n\
    upgrade: websocket\r\n\
    connection: upgrade\r\n\
    sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
    sec-websocket-version: 13\r\n\r\n";

    pub const RESPONSE: &[u8] = b"\
        HTTP/1.1 101 Switching Protocols\r\n\
        upgrade: websocket\r\n\
        connection: upgrade\r\n\
        sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";

    pub struct LimitReadWriter {
        pub rbuf: Vec<u8>,
        pub wbuf: Vec<u8>,
        pub rlimit: usize,
        pub wlimit: usize,
        pub cursor: usize,
    }

    impl Read for LimitReadWriter {
        fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
            let to_read = std::cmp::min(buf.len(), self.rlimit);
            let left_data = self.rbuf.len() - self.cursor;
            if left_data == 0 {
                return Ok(0);
            }
            if left_data <= to_read {
                buf.write(&self.rbuf[self.cursor..]).unwrap();
                self.cursor = self.rbuf.len();
                return Ok(left_data);
            }

            buf.write(&self.rbuf[self.cursor..self.cursor + to_read])
                .unwrap();
            self.cursor += to_read;
            Ok(to_read)
        }
    }

    impl Write for LimitReadWriter {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            let len = std::cmp::min(buf.len(), self.wlimit);
            self.wbuf.write(&buf[..len])
        }

        fn flush(&mut self) -> Result<()> { Ok(()) }
    }
}
