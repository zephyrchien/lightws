use super::Stream;
use std::io::Result;
use std::net::TcpStream;

impl<Role> Stream<TcpStream, Role> {
    /// Creates a new independently owned handle to the underlying IO source.
    /// Caution: **states are not shared among instances.**
    pub fn try_clone(&self) -> Result<Self> {
        let io = self.io.try_clone()?;
        Ok(Self::new(io))
    }
}
