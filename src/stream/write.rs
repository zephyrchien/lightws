use std::io::{Write, Result};

use super::{Stream, RoleHelper};
use super::detail::write_some;

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
        write_some(self, |io, iovec| io.write_vectored(iovec), buf)
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
    use super::super::test::{LimitReadWriter, make_frame};
    use crate::frame::*;
    use crate::role::*;
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
