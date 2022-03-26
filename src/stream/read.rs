use std::io::Read;
use std::io::Result;

use super::Stream;
use super::RoleHelper;
use super::ReadState;
use super::state::HeadStore;
use super::common::min_len;

use crate::frame::{OpCode, Mask, FrameHead};
use crate::frame::mask::apply_mask4;
use crate::error::FrameError;

impl<IO: Read, Role: RoleHelper> Read for Stream<IO, Role> {
    /// Read some data from the underlying IO source,
    /// returns `Ok(0)` until a complete frame head is present.
    /// Caller should ensure the available buffer size is larger
    /// than **14** before a read.
    ///
    /// Read a control frame(like Ping) returns `Ok(0)`,
    /// which could be detected via [`Stream::is_pinged`].
    ///
    /// Any read after receiving a `Close` frame or reaching `EOF`
    /// will return `Ok(0)`,
    /// which could be checked via [`Stream::is_read_end`],
    /// [`Stream::is_read_close`], [`Stream::is_read_eof`].
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        debug_assert!(buf.len() >= 14);

        loop {
            match self.read_state {
                // always returns 0
                ReadState::Eof => return Ok(0),
                ReadState::Close => return Ok(0),
                // read a new incoming frame
                ReadState::ReadHead(head_store) => {
                    let head_store_len = head_store.rd_left();

                    // write stored data to user provided buffer
                    if !head_store.is_empty() {
                        let (left, _) = buf.split_at_mut(head_store_len);
                        left.copy_from_slice(head_store.read());
                    }

                    let read_n = self.io.read(&mut buf[head_store_len..])?;

                    // EOF ?
                    if read_n == 0 {
                        self.read_state = ReadState::Eof;
                        return Ok(0);
                    }

                    self.read_state = ReadState::ProcessBuf {
                        beg: 0,
                        end: read_n + head_store_len,
                        processed: 0,
                    }
                }
                // continue to read data from the same frame
                ReadState::ReadData { next, mask } => {
                    let len = min_len(buf.len(), next);
                    let read_n = self.io.read(&mut buf[..len])?;
                    // EOF ?
                    if read_n == 0 {
                        self.read_state = ReadState::Eof;
                        return Ok(0);
                    }
                    // unmask if server receives data from client
                    // this operation can be skipped if mask key is 0
                    if let Mask::Key(key) = mask {
                        apply_mask4(key, &mut buf[..len])
                    };
                    // read complete ?
                    if next == read_n as u64 {
                        self.read_state = ReadState::new();
                    } else {
                        self.read_state = ReadState::ReadData {
                            next: next - read_n as u64,
                            mask,
                        };
                    }
                    return Ok(read_n);
                }
                // continue to read data from a ctrl frame
                ReadState::ReadPing { next, mask } => {
                    let (buf, _) = self
                        .heartbeat
                        .ping_store
                        .write()
                        .split_at_mut(next as usize);
                    let read_n = self.io.read(buf)?;
                    // EOF ?
                    if read_n == 0 {
                        self.read_state = ReadState::Eof;
                        return Ok(0);
                    }
                    // unmask if server receives data from client
                    // this operation can be skipped if mask key is 0
                    if let Mask::Key(key) = mask {
                        apply_mask4(key, buf);
                    };

                    self.heartbeat.ping_store.advance_wr_pos(read_n);

                    // read complete ?
                    if next == read_n as u8 {
                        self.heartbeat.is_complete = true;
                        self.read_state = ReadState::new();
                    } else {
                        self.read_state = ReadState::ReadPing {
                            next: next - read_n as u8,
                            mask,
                        };
                    }
                    return Ok(0);
                }
                // handle the read data in user provided buffer
                ReadState::ProcessBuf {
                    mut beg,
                    end,
                    mut processed,
                } => {
                    // parse head, fin is ignored
                    let (
                        FrameHead {
                            opcode,
                            mask,
                            length,
                            ..
                        },
                        parse_n,
                    ) = match FrameHead::decode(&buf[beg..end]) {
                        Ok(x) => x,
                        Err(ref e) if *e == FrameError::NotEnoughData => {
                            if beg == end {
                                self.read_state = ReadState::new();
                            } else {
                                self.read_state =
                                    ReadState::ReadHead(HeadStore::new_with_data(&buf[beg..end]));
                            }
                            return Ok(processed);
                        }
                        Err(e) => return Err(e.into()),
                    };
                    // point to payload
                    beg += parse_n;

                    // may read a frame without payload
                    let frame_len = length.to_num();
                    let buf_len = end - beg;
                    let data_len = min_len(buf_len, frame_len);

                    match opcode {
                        // text is not allowed
                        // we never send a ping, so we ignore the pong
                        OpCode::Text | OpCode::Pong => {
                            return Err(FrameError::UnsupportedOpcode.into())
                        }
                        // ignore fin flag
                        OpCode::Binary | OpCode::Continue => {
                            if data_len != 0 {
                                // unmask payload data from client
                                if let Mask::Key(key) = mask {
                                    apply_mask4(key, &mut buf[beg..beg + data_len]);
                                }
                                // move forward
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        buf.as_ptr().add(beg),
                                        buf.as_mut_ptr().add(processed),
                                        data_len,
                                    );
                                };
                            }
                            beg += data_len;
                            processed += data_len;
                            // need to read more payload
                            if frame_len > buf_len as u64 {
                                self.read_state = ReadState::ReadData {
                                    next: frame_len - data_len as u64,
                                    mask,
                                };
                                return Ok(processed);
                            }
                            // continue to process
                            self.read_state = ReadState::ProcessBuf {
                                beg,
                                end,
                                processed,
                            };
                        }
                        OpCode::Ping => {
                            // a ping frame must not have extened data
                            if frame_len > 125 {
                                return Err(FrameError::IllegalData.into());
                            }
                            if data_len != 0 {
                                // unmask payload data from client
                                if let Mask::Key(key) = mask {
                                    apply_mask4(key, &mut buf[beg..beg + data_len]);
                                }
                                // save ping data
                                self.heartbeat
                                    .ping_store
                                    .replace_with_data(&buf[beg..beg + data_len]);
                            } else {
                                // no payload
                                self.heartbeat.ping_store.reset();
                            }

                            // processed does not increase;
                            beg += data_len;

                            // need to read more payload
                            if frame_len > buf_len as u64 {
                                self.heartbeat.is_complete = false;
                                self.read_state = ReadState::ReadPing {
                                    next: frame_len as u8 - data_len as u8,
                                    mask,
                                };
                                return Ok(processed);
                            }
                            // continue to process
                            self.heartbeat.is_complete = true;
                            self.read_state = ReadState::ProcessBuf {
                                beg,
                                end,
                                processed,
                            };
                        }
                        OpCode::Close => {
                            self.read_state = ReadState::Close;
                            return Ok(processed);
                        }
                    }
                }
            }
        }
    }

    /// Override default implement, exit when reaching `EOF`
    /// or receiving a `Close` frame.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        use std::io::ReadBuf;
        use std::io::ErrorKind;

        let start_len = buf.len();
        let start_cap = buf.capacity();

        let mut initialized = 0; // Extra initialized bytes from previous loop iteration
        loop {
            if buf.len() < buf.capacity() + 14 {
                buf.reserve(32); // buf is full, need more space
            }

            let mut read_buf = ReadBuf::uninit(buf.spare_capacity_mut());

            // SAFETY: These bytes were initialized but not filled in the previous loop
            unsafe {
                read_buf.assume_init(initialized);
            }

            match self.read_buf(&mut read_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }

            if read_buf.filled_len() == 0 && self.is_read_end() {
                return Ok(buf.len() - start_len);
            }

            // store how much was initialized but not filled
            initialized = read_buf.initialized_len() - read_buf.filled_len();
            let new_len = read_buf.filled_len() + buf.len();

            // SAFETY: ReadBuf's invariants mean this much memory is init
            unsafe {
                buf.set_len(new_len);
            }

            if buf.len() == buf.capacity() && buf.capacity() == start_cap {
                // The buffer might be an exact fit. Let's read into a probe buffer
                // and see if it returns `Ok(0)`. If so, we've avoided an
                // unnecessary doubling of the capacity. But if not, append the
                // probe buffer to the primary buffer and let its capacity grow.
                let mut probe = [0u8; 32];

                loop {
                    match self.read(&mut probe) {
                        Ok(0) => {
                            if self.is_read_end() {
                                return Ok(buf.len() - start_len);
                            }
                        }
                        Ok(n) => {
                            buf.extend_from_slice(&probe[..n]);
                            break;
                        }
                        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }

    /// **This is NOT supported!**
    fn read_exact(&mut self, _: &mut [u8]) -> Result<()> {
        panic!("Unsupported");
    }

    /// **This is NOT supported!**
    fn read_to_string(&mut self, _: &mut String) -> Result<usize> {
        panic!("Unsupported");
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;
    use super::*;
    use super::super::{Client, Server};
    use super::super::test::{LimitReadWriter, make_frame};
    use crate::frame::*;

    #[test]
    fn read_from_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(n: usize) {
            let (frame, data) = make_frame::<R1>(OpCode::Binary, n);

            let mut stream = Stream::<_, R2>::new(frame.as_slice());

            let mut buf = vec![0; n + 14];
            let read_n = stream.read(&mut buf).unwrap();

            assert_eq!(read_n, n);
            assert_eq!(&buf[..n], &data);
        }

        for i in 0..=0x2000 {
            read::<Client, Server>(i);
            read::<Server, Client>(i);
        }

        for i in [65536, 65537, 100000] {
            read::<Client, Server>(i);
            read::<Server, Client>(i);
        }
    }

    #[test]
    fn read_from_limit_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(n: usize, limit: usize) {
            let (frame, data) = make_frame::<R1>(OpCode::Binary, n);

            let io = LimitReadWriter {
                buf: frame,
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = Vec::new();
            let mut stream = Stream::<_, R2>::new(io);

            let read_n = stream.read_to_end(&mut buf).unwrap();

            assert_eq!(read_n, n);
            assert_eq!(&buf[..n], &data);
        }

        for i in 0..=256 {
            for limit in 1..=300 {
                read::<Client, Server>(i, limit);
                read::<Server, Client>(i, limit);
            }
        }

        for i in [65536, 65537, 100000] {
            for limit in 1..=1024 {
                read::<Client, Server>(i, limit);
                read::<Server, Client>(i, limit);
            }
        }
    }

    #[test]
    fn read_eof_from_stream() {
        fn read<R: RoleHelper>() {
            let io = LimitReadWriter {
                buf: b"EOFFFF:)".to_vec(),
                rlimit: 0,
                wlimit: 0,
                cursor: 0,
            };
            let mut stream = Stream::<_, R>::new(io);
            let mut buf = vec![0; 32];
            let n = stream.read(&mut buf).unwrap();
            assert_eq!(n, 0);
            assert!(stream.is_read_end());
            assert!(stream.is_read_eof());

            let n = stream.read_to_end(&mut buf).unwrap();
            assert_eq!(n, 0);
            assert!(stream.is_read_end());
            assert!(stream.is_read_eof());
        }
        read::<Client>();
        read::<Server>();
    }

    #[test]
    fn read_close_from_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(limit: usize) {
            let (frame, _) = make_frame::<R1>(OpCode::Close, 1);
            let io = LimitReadWriter {
                buf: frame,
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut stream = Stream::<_, R2>::new(io);

            let mut buf = vec![0; 32];

            let n = stream.read(&mut buf).unwrap();
            assert_eq!(n, 0);

            let n = stream.read_to_end(&mut buf).unwrap();
            assert_eq!(n, 0);
            assert!(stream.is_read_end());
            assert!(stream.is_read_close());
        }

        for i in 1..=32 {
            read::<Client, Server>(i);
            read::<Server, Client>(i);
        }
    }

    #[test]
    fn read_ping_from_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(n: usize, limit: usize) {
            let (frame, data) = make_frame::<R1>(OpCode::Ping, n);

            let io = LimitReadWriter {
                buf: frame,
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = Vec::new();
            let mut stream = Stream::<_, R2>::new(io);

            let read_n = stream.read_to_end(&mut buf).unwrap();

            assert_eq!(read_n, 0);
            assert_eq!(stream.ping_data(), &data);
        }

        for i in 0..=125 {
            for limit in 1..=128 {
                read::<Client, Server>(i, limit);
                read::<Server, Client>(i, limit);
            }
        }
    }

    #[test]
    fn read_multi_frame_from_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(n: usize, step: usize, limit: usize) {
            let mut len = 0;
            let mut frame = Vec::new();
            let mut data = Vec::new();

            for i in 0..n {
                let (mut f, mut d) = make_frame::<R1>(OpCode::Binary, step + i * step);
                len += d.len();
                frame.append(&mut f);
                data.append(&mut d);
                assert_eq!(len, (i + 1) * (i + 2) * step / 2);
            }

            let (mut close, _) = make_frame::<R1>(OpCode::Close, 1);
            frame.append(&mut close);

            let io = LimitReadWriter {
                buf: frame,
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = Vec::new();
            let mut stream = Stream::<_, R2>::new(io);

            let read_n = stream.read_to_end(&mut buf).unwrap();

            assert!(stream.is_read_end());
            assert!(stream.is_read_close());
            assert_eq!(read_n, len);
            assert_eq!(&buf[..len], &data);
        }

        for n in 1..=20 {
            for step in [1, 10, 100, 1000, 10000] {
                for limit in [1, 10, 100, 1000, 10000, usize::MAX] {
                    read::<Client, Server>(n, step, limit);
                    read::<Server, Client>(n, step, limit);
                }
            }
        }
    }

    #[test]
    fn read_multi_ping_from_stream() {
        fn read<R1: RoleHelper, R2: RoleHelper>(n: usize, step: usize, limit: usize) {
            let mut len = 0;
            let mut frame = Vec::new();
            let mut data = Vec::new();

            for i in 0..n {
                let (mut f, d) = make_frame::<R1>(OpCode::Ping, step + i * step);
                len += d.len();
                frame.append(&mut f);
                data = d;
                assert_eq!(len, (i + 1) * (i + 2) * step / 2);
            }

            let io = LimitReadWriter {
                buf: frame,
                rlimit: limit,
                wlimit: 0,
                cursor: 0,
            };

            let mut buf = Vec::new();
            let mut stream = Stream::<_, R2>::new(io);

            let read_n = stream.read_to_end(&mut buf).unwrap();

            assert_eq!(read_n, 0);
            assert_eq!(stream.ping_data(), &data);
        }

        for n in 1..=125 {
            for limit in 1..=128 {
                read::<Client, Server>(n, 1, limit);
                read::<Server, Client>(n, 1, limit);
            }
        }
    }
}
