use std::io::Result;
use std::pin::Pin;
use std::task::{Poll, Context};

use tokio::io::AsyncWrite;

use super::{Stream, RoleHelper, Guarded};
use super::detail::write_some;

impl<IO, Role> AsyncWrite for Stream<IO, Role>
where
    IO: AsyncWrite + Unpin,
    Stream<IO, Role>: Unpin,
    Role: RoleHelper,
{
    /// Async version of `Stream::write`.
    #[rustfmt::skip]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        write_some(self.get_mut(), |io, buf| Pin::new(io).poll_write_vectored(cx, buf), buf)
    }

    /// This is a no-op since we do not buffer any data.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_flush(cx)
    }

    /// Shutdown the underlying IO source.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_shutdown(cx)
    }
}

impl<IO, Role> AsyncWrite for Stream<IO, Role, Guarded>
where
    IO: AsyncWrite + Unpin,
    Stream<IO, Role, Guarded>: Unpin,
    Role: RoleHelper,
{
    /// Async version of `Stream::write`.
    /// Continue to write if frame head is not completely written.
    #[rustfmt::skip]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        loop {
            match write_some(this, |io, buf| Pin::new(io).poll_write_vectored(cx, buf), buf) {
                Poll::Ready(Ok(0)) if this.is_write_partial_head() || !this.is_write_zero()=> continue,
                Poll::Ready(Ok(n)) => return Poll::Ready(Ok(n)),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    /// This is a no-op since we do not buffer any data.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_flush(cx)
    }

    /// Shutdown the underlying IO source.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_shutdown(cx)
    }
}
