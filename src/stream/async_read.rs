use std::io::Result;
use std::pin::Pin;
use std::task::{Poll, Context};

use tokio::io::AsyncRead;
use tokio::io::ReadBuf;

use super::{Stream, RoleHelper};
use super::detail::read_some;

impl<IO, Role> AsyncRead for Stream<IO, Role>
where
    IO: AsyncRead + Unpin,
    Stream<IO, Role>: Unpin,
    Role: RoleHelper,
{
    /// Async version of `Stream::read`.
    #[rustfmt::skip]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        read_some(self.get_mut(), |io, buf| {
                let mut buf = ReadBuf::new(buf);
                Pin::new(io).poll_read(cx, &mut buf)
                .map_ok(|_| buf.filled().len())
            },
            buf.initialize_unfilled(),
        ).map_ok(|n| buf.advance(n))
    }
}
