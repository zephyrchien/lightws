//! Websocket stream.
//!
//! [`Stream`] is a simple wrapper of the underlying IO source,
//! with small stack buffers to save states.
//!
//! It is transparent to call `Read` or `Write` on Stream:
//!
//! ```ignore
//! {
//!     // establish connection, handshake
//!     let stream = ...
//!     // read some data
//!     stream.read(&mut buf)?;
//!     // write some data
//!     stream.write(&buf)?;
//! }
//! ```
//!
//! A new established [`Stream`] is in [`Direct`] (default) mode, where
//! a `Read` or `Write` leads to **at most one** syscall, and
//! an `Ok(0)` will be returned if frame head is not completely read or written.
//! It can be converted to [`Guarded`] mode with [`Stream::guard`],
//! which wraps `Read` or `Write` in a loop, where `Ok(0)` is handled internally.
//!
//! Stream itself does not buffer any payload data during
//! a `Read` or `Write`, so there is no extra heap allocation.
//!
//! # Masking payload
//!
//! Data read from stream are automatically unmasked.
//! However, data written to stream are **NOT** automatically masked,
//! since a `Write` call requires an immutable `&[u8]`.
//!
//! A standard client(e.g. [`StandardClient`](crate::role::StandardClient))
//! should mask the payload before sending it;
//! A non-standard client (e.g. [`Client`](crate::role::Client)) which holds an empty mask key
//! can simply skip this step.
//!
//! The mask key is prepared by [`ClientRole`](crate::role::ClientRole),
//! which can be set or fetched via [`Stream::set_mask_key`] and [`Stream::mask_key`].
//!
//! Example:
//!
//! ```no_run
//! use std::io::{Read, Write};
//! use std::net::TcpStream;
//! use lightws::role::StandardClient;
//! use lightws::endpoint::Endpoint;
//! use lightws::frame::{new_mask_key, apply_mask4};
//! fn write_data() -> std::io::Result<()> {  
//!     let mut buf = [0u8; 256];
//!     let mut tcp = TcpStream::connect("example.com:80")?;
//!     let mut ws = Endpoint::<TcpStream, StandardClient>::connect(tcp, &mut buf, "example.com", "/ws")?;
//!
//!     // mask data
//!     let key = new_mask_key();
//!     apply_mask4(key, &mut buf);
//!
//!     // set mask key for next write
//!     ws.set_mask_key(key)?;
//!
//!     // write some data
//!     ws.write_all(&buf)?;
//!     Ok(())
//! }
//! ```
//!
//! # Automatic masking
//!
//! It is annoying to mask the payload each time before a write,
//! and it will block us from using convenient functions like [`std::io::copy`].
//!
//! With `unsafe_auto_mask_write` fearure enabled, the provided immutable `&[u8]` will be casted
//! to a mutable `&mut [u8]` then payload data can be automatically masked.
//!
//! This feature only has effects on [`AutoMaskClientRole`](crate::role::AutoMaskClientRole),
//! where its inner mask key may be updated (depends on
//! [`AutoMaskClientRole::UPDATE_MASK_KEY`](crate::role::AutoMaskClientRole::UPDATE_MASK_KEY))
//! and used to mask the payload before each write.
//! Other [`ClientRole`](crate::role::ClientRole) and [`ServerRole`](crate::role::ServerRole)
//! are not affected. Related code lies in `src/stream/detail/write#L118`.
//!

mod read;
mod write;

mod ctrl;
mod state;
mod detail;
mod special;

cfg_if::cfg_if! {
    if #[cfg(feature = "async")] {
        mod async_read;
        mod async_write;
    }
}

use std::marker::PhantomData;
use state::{ReadState, WriteState, HeartBeat};
use crate::role::RoleHelper;

/// Direct read or write.
pub struct Direct {}

/// Wrapped read or write.
pub struct Guarded {}

/// Websocket stream.
///
/// Depending on `IO`, [`Stream`] implements [`std::io::Read`] and [`std::io::Write`]
/// or [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`].
///
/// `Role` decides whether to mask payload data.
/// It is reserved to provide extra infomation to apply optimizations.
///
/// See also: `Stream::read`, `Stream::write`.
pub struct Stream<IO, Role, Guard = Direct> {
    io: IO,
    role: Role,
    read_state: ReadState,
    write_state: WriteState,
    heartbeat: HeartBeat,
    __marker: PhantomData<Guard>,
}

impl<IO, Role, Guard> AsRef<IO> for Stream<IO, Role, Guard> {
    #[inline]
    fn as_ref(&self) -> &IO { &self.io }
}

impl<IO, Role, Guard> AsMut<IO> for Stream<IO, Role, Guard> {
    #[inline]
    fn as_mut(&mut self) -> &mut IO { &mut self.io }
}

impl<IO, Role, Guard> std::fmt::Debug for Stream<IO, Role, Guard> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("read_state", &self.read_state)
            .field("write_state", &self.write_state)
            .field("heartbeat", &self.heartbeat)
            .finish()
    }
}

impl<IO, Role> Stream<IO, Role> {
    /// Create websocket stream from IO source directly,
    /// without a handshake.
    #[inline]
    pub const fn new(io: IO, role: Role) -> Self {
        Stream {
            io,
            role,
            read_state: ReadState::new(),
            write_state: WriteState::new(),
            heartbeat: HeartBeat::new(),
            __marker: PhantomData,
        }
    }

    /// Convert to a guarded stream.
    #[inline]
    pub fn guard(self) -> Stream<IO, Role, Guarded> {
        Stream {
            io: self.io,
            role: self.role,
            read_state: self.read_state,
            write_state: self.write_state,
            heartbeat: self.heartbeat,
            __marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Read, Write, Result};
    use crate::frame::*;
    use crate::role::*;

    pub struct LimitReadWriter {
        pub buf: Vec<u8>,
        pub rlimit: usize,
        pub wlimit: usize,
        pub cursor: usize,
    }

    impl Read for LimitReadWriter {
        fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
            let to_read = std::cmp::min(buf.len(), self.rlimit);
            let left_data = self.buf.len() - self.cursor;
            if left_data == 0 {
                return Ok(0);
            }
            if left_data <= to_read {
                buf.write(&self.buf[self.cursor..]).unwrap();
                self.cursor = self.buf.len();
                return Ok(left_data);
            }

            buf.write(&self.buf[self.cursor..self.cursor + to_read])
                .unwrap();
            self.cursor += to_read;
            Ok(to_read)
        }
    }

    impl Write for LimitReadWriter {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            let len = std::cmp::min(buf.len(), self.wlimit);
            self.buf.write(&buf[..len])
        }

        fn flush(&mut self) -> Result<()> { Ok(()) }
    }

    pub fn make_head(opcode: OpCode, mask: Mask, len: usize) -> Vec<u8> {
        let mut tmp = vec![0; 14];
        let head = FrameHead::new(Fin::Y, opcode, mask, PayloadLen::from_num(len as u64));

        let head_len = head.encode(&mut tmp).unwrap();
        let mut head = Vec::new();
        let write_n = head.write(&tmp[..head_len]).unwrap();
        assert_eq!(write_n, head_len);
        head
    }

    pub fn make_data(len: usize) -> Vec<u8> {
        std::iter::repeat(rand::random::<u8>()).take(len).collect()
    }

    pub fn make_frame<R: RoleHelper>(opcode: OpCode, len: usize) -> (Vec<u8>, Vec<u8>) {
        make_frame_with_mask(opcode, R::new().mask_key(), len)
    }

    // data is unmasked
    pub fn make_frame_with_mask(opcode: OpCode, mask: Mask, len: usize) -> (Vec<u8>, Vec<u8>) {
        let data = make_data(len);
        let mut data2 = data.clone();

        let mut frame = make_head(opcode, mask, len);
        let head_len = frame.len();

        frame.append(&mut data2);
        assert_eq!(frame.len(), len + head_len);

        (frame, data)
    }

    #[test]
    fn read_write_stream() {
        fn read_write<R: RoleHelper>(rlimit: usize, wlimit: usize, len: usize) {
            let io = LimitReadWriter {
                buf: Vec::new(),
                rlimit,
                wlimit,
                cursor: 0,
            };
            // data written to a client stream should be read as a server stream.
            // here we read/write on the same (client/server)stream.
            // this is not correct in practice, but our program can still handle it.
            let mut stream = Stream::<_, R>::new(io, R::new());

            let data: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(len).collect();
            let mut data2: Vec<u8> = Vec::new();

            let mut buf = vec![0; 0x2000];
            let mut to_write = data.len();

            while to_write > 0 {
                let wbeg = data.len() - to_write;
                let n = loop {
                    let x = stream.write(&data[wbeg..]).unwrap();
                    if x != 0 {
                        break x;
                    }
                };

                let mut tmp: Vec<u8> = Vec::new();
                loop {
                    // avoid read EOF here
                    if stream.as_ref().cursor == stream.as_ref().buf.len() {
                        break;
                    }
                    let n = stream.read(&mut buf).unwrap();

                    // if n == 0 && stream.is_read_end() {
                    //     break;
                    // }

                    tmp.write(&buf[..n]).unwrap();
                }

                assert_eq!(tmp.len(), n);
                assert_eq!(&data[wbeg..wbeg + n], &tmp);

                to_write -= n;
                data2.append(&mut tmp);
            }

            assert_eq!(&data, &data2);
        }

        for limit in 1..512 {
            for len in 1..=256 {
                read_write::<Client>(limit, 512 - limit, len);
                read_write::<Server>(limit, 512 - limit, len);
            }
        }
    }
}
