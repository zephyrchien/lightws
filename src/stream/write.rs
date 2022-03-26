use std::io::Write;
use std::io::Result;
use std::io::IoSlice;

use super::Stream;
use super::RoleHelper;
use super::WriteState;
use super::common::{min_len, write_data_frame};

impl<IO: Write, Role: RoleHelper> Write for Stream<IO, Role> {
    /// Write some data to the underlying IO source,
    /// returns `Ok(0)` until the frame head is completely
    /// written.
    ///
    /// if `WriteZero` occurs, it will also return `Ok(0)`,
    /// which could be detected via [`Stream::is_write_zero`].
    ///
    /// Frame head will be generated automatically,
    /// according to the length of the provided buffer.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self.write_state {
            // always returns 0
            WriteState::WriteZero => Ok(0),
            // create a new frame
            WriteState::WriteHead(mut head_store) => {
                // data frame length depends on provided buffer length
                let frame_len = buf.len();

                if head_store.is_empty() {
                    write_data_frame::<Role>(&mut head_store, frame_len as u64);
                }
                // frame head(maybe partial) + payload
                let iovec = [IoSlice::new(head_store.read()), IoSlice::new(buf)];
                let write_n = self.io.write_vectored(&iovec)?;
                let head_len = head_store.rd_left() as usize;

                // write zero ?
                if write_n == 0 {
                    self.write_state = WriteState::WriteZero;
                    return Ok(0);
                }

                // frame head is not written completely
                if write_n < head_len {
                    head_store.advance_rd_pos(write_n);
                    self.write_state = WriteState::WriteHead(head_store);
                    return Ok(0);
                }

                // frame has been written completely
                let write_n = write_n - head_len;

                // all data written ?
                if write_n == frame_len {
                    self.write_state = WriteState::new();
                } else {
                    self.write_state = WriteState::WriteData((frame_len - write_n) as u64);
                }

                Ok(write_n)
            }
            // continue to write to the same frame
            WriteState::WriteData(next) => {
                let len = min_len(buf.len(), next);
                let write_n = self.io.write(&buf[..len])?;
                // write zero ?
                if write_n == 0 {
                    self.write_state = WriteState::WriteZero;
                    return Ok(0);
                }
                // all data written ?
                if next == write_n as u64 {
                    self.write_state = WriteState::new()
                } else {
                    self.write_state = WriteState::WriteData(next - write_n as u64)
                }
                Ok(write_n)
            }
        }
    }

    /// The writer does not buffer any data, simply flush
    /// the underlying IO source.
    fn flush(&mut self) -> Result<()> { self.io.flush() }

    /// Override the default implement, allowing `Ok(0)` if
    /// the frame head is not completely written.
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        use std::io::ErrorKind;
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    if self.is_write_zero() {
                        return Err(ErrorKind::WriteZero.into());
                    }
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{Client, Server, RoleHelper};
    use super::super::test::{LimitReadWriter, make_frame};
    use crate::frame::*;
    use std::io::Write;

    #[test]
    fn write_to_stream() {
        fn write<R: RoleHelper>(n: usize) {
            let (frame, data) = make_frame::<R>(OpCode::Binary, n);

            let io: Vec<u8> = Vec::new();
            let mut stream = Stream::<_, R>::new(io);

            let write_n = stream.write(&data).unwrap();

            assert_eq!(write_n, n);

            assert_eq!(stream.as_ref(), &frame);
        }

        for i in 1..=0x2000 {
            write::<Client>(i);
            write::<Server>(i);
        }

        for i in [65536, 65537, 100000] {
            write::<Client>(i);
            write::<Server>(i);
        }
    }

    #[test]
    fn write_to_limit_stream() {
        fn write<R: RoleHelper>(n: usize, limit: usize) {
            let (frame, data) = make_frame::<R>(OpCode::Binary, n);

            let io = LimitReadWriter {
                buf: Vec::new(),
                rlimit: 0,
                wlimit: limit,
                cursor: 0,
            };

            let mut stream = Stream::<_, R>::new(io);

            stream.write_all(&data).unwrap();

            assert_eq!(&stream.as_ref().buf, &frame);
        }

        for i in 1..=256 {
            for limit in 1..=300 {
                write::<Client>(i, limit);
                write::<Server>(i, limit);
            }
        }

        for i in [65536, 65537, 100000] {
            for limit in 1..=1024 {
                write::<Client>(i, limit);
                write::<Server>(i, limit);
            }
        }
    }
}
