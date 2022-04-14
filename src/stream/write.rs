use std::io::{Write, Result};
use std::task::Poll;

use super::{Stream, RoleHelper, Guarded};
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
    ///
    /// A standard client should mask payload data before sending it.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match write_some(self, |io, iovec| io.write_vectored(iovec).into(), buf) {
            Poll::Ready(x) => x,
            Poll::Pending => unreachable!(),
        }
    }

    /// The writer does not buffer any data, simply flush
    /// the underlying IO source.
    fn flush(&mut self) -> Result<()> { self.io.flush() }

    /// **This is NOT supported!**
    fn write_all(&mut self, _: &[u8]) -> Result<()> {
        panic!("Unsupported");
    }
}

impl<IO: Write, Role: RoleHelper> Write for Stream<IO, Role, Guarded> {
    /// Wrap write in a loop.
    /// Continue to write if frame head is not completely written.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        loop {
            match write_some(self, |io, iovec| io.write_vectored(iovec).into(), buf) {
                Poll::Ready(Ok(0)) if self.is_write_partial_head() || !self.is_write_zero() => {
                    continue
                }
                Poll::Ready(x) => return x,
                Poll::Pending => unreachable!(),
            }
        }
    }

    /// The writer does not buffer any data, simply flush
    /// the underlying IO source.
    fn flush(&mut self) -> Result<()> { self.io.flush() }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::test::*;
    use crate::frame::*;
    use crate::role::*;
    use std::io::Write;

    #[test]
    fn write_to_stream() {
        fn write<R: RoleHelper>(n: usize) {
            let (frame, data) = make_frame::<R>(OpCode::Binary, n);

            let io: Vec<u8> = Vec::new();
            let mut stream = Stream::new(io, R::new());

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

            let mut stream = Stream::new(io, R::new()).guard();

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

    #[test]
    #[cfg(feature = "unsafe_auto_mask_write")]
    fn write_to_stream_auto_mask_fixed() {
        fn write<R: RoleHelper>(n: usize) {
            let key = new_mask_key();

            let (mut frame, data) = make_frame_with_mask(OpCode::Binary, Mask::Key(key), n);

            // manually mask frame data
            let offset = frame.len() - n;
            apply_mask4(key, &mut frame[offset..]);

            let io: Vec<u8> = Vec::new();
            let mut stream = Stream::new(io, R::new());
            stream.set_write_mask_key(key).unwrap();

            let write_n = stream.write(&data).unwrap();

            assert_eq!(write_n, n);

            assert_eq!(stream.as_ref(), &frame);
        }
        for i in 1..=2 {
            write::<FixedMaskClient>(i);
        }

        for i in [65536, 65537, 100000] {
            write::<FixedMaskClient>(i);
        }
    }

    #[test]
    #[cfg(feature = "unsafe_auto_mask_write")]
    fn write_to_limit_stream_auto_mask_fixed() {
        fn write<R: RoleHelper>(n: usize, limit: usize) {
            let key = new_mask_key();
            let (mut frame, data) = make_frame_with_mask(OpCode::Binary, Mask::Key(key), n);

            // manually mask frame data
            let offset = frame.len() - n;
            apply_mask4(key, &mut frame[offset..]);

            let io = LimitReadWriter {
                buf: Vec::new(),
                rlimit: 0,
                wlimit: limit,
                cursor: 0,
            };

            let mut stream = Stream::new(io, R::new()).guard();
            stream.set_write_mask_key(key).unwrap();

            stream.write_all(&data).unwrap();

            assert_eq!(&stream.as_ref().buf, &frame);
        }

        for i in 1..=256 {
            for limit in 1..=300 {
                write::<FixedMaskClient>(i, limit);
            }
        }

        for i in [65536, 65537, 100000] {
            for limit in 1..=1024 {
                write::<FixedMaskClient>(i, limit);
            }
        }
    }

    #[test]
    #[cfg(feature = "unsafe_auto_mask_write")]
    fn write_to_stream_auto_mask_updated() {
        fn write<R: RoleHelper>(n: usize) {
            let data = make_data(n);
            let mut data2 = data.clone();

            let io: Vec<u8> = Vec::new();
            let mut stream = Stream::new(io, R::new());

            let write_n = stream.write(&data).unwrap();
            assert_eq!(write_n, n);

            // manually mask frame data
            let key = stream.write_mask_key().to_key();
            let head = make_head(OpCode::Binary, Mask::Key(key), n);
            apply_mask4(key, &mut data2);

            assert_eq!(stream.as_ref()[..head.len()], head);
            assert_eq!(stream.as_ref()[head.len()..], data2);
        }
        for i in 1..=2 {
            write::<StandardClient>(i);
        }

        for i in [65536, 65537, 100000] {
            write::<StandardClient>(i);
        }
    }

    #[test]
    #[cfg(feature = "unsafe_auto_mask_write")]
    fn write_to_limit_stream_auto_mask_updated() {
        fn write<R: RoleHelper>(n: usize, limit: usize) {
            let data = make_data(n);
            let mut data2 = data.clone();

            let io = LimitReadWriter {
                buf: Vec::new(),
                rlimit: 0,
                wlimit: limit,
                cursor: 0,
            };

            let mut stream = Stream::new(io, R::new()).guard();
            stream.write_all(&data).unwrap();

            // manually mask frame data
            let key = stream.write_mask_key().to_key();
            let head = make_head(OpCode::Binary, Mask::Key(key), n);
            apply_mask4(key, &mut data2);

            assert_eq!(stream.as_ref().buf[..head.len()], head);
            assert_eq!(stream.as_ref().buf[head.len()..], data2);
        }

        for i in 1..=256 {
            for limit in 1..=300 {
                write::<StandardClient>(i, limit);
            }
        }

        for i in [65536, 65537, 100000] {
            for limit in 1..=1024 {
                write::<StandardClient>(i, limit);
            }
        }
    }
}
