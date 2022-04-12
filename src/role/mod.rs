//! Markers.
//!
//! Markers are used to apply different strategies as a client or server.
//!
//! For example, `Endpoint<IO, Client>::connect` is used to to open a connection,
//! and returns `Stream<IO, Client>`; `Endpoint<IO, Server>` is used to accept
//! a connection and returns `Stream<IO, Server>`.
//!
//! Both client and server meet [`RoleHelper`], which indicates frame head length
//! (currently unused), and how to mask payload data. Only client meets [`ClientRole`],
//! and only server meets [`ServerRole`].
//!
//! Any type implements these traits will be treated as a `client` or `server`.

use crate::frame::Mask;

/// Client or Server marker.
pub trait RoleHelper: Clone + Copy {
    const SHORT_FRAME_HEAD_LEN: u8;
    const COMMON_FRAME_HEAD_LEN: u8;
    const LONG_FRAME_HEAD_LEN: u8;

    fn new() -> Self;
    fn write_mask(&self) -> Mask;
    // by default this is a no-op
    fn set_write_mask(&mut self, _: [u8; 4]) {}
}

/// Client marker.
pub trait ClientRole: RoleHelper {}

/// Server marker.
pub trait ServerRole: RoleHelper {}

/// Simple client using empty(fake) mask key.
///
/// With an empty mask key, the sender/receiver
/// does not need to mask/unmask the payload.
#[derive(Clone, Copy)]
pub struct Client;

/// Standard server.
#[derive(Clone, Copy)]
pub struct Server;

/// Standard client using random mask key.
#[derive(Clone, Copy)]
pub struct StandardClient([u8; 4]);

impl RoleHelper for Client {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;

    #[inline]
    fn new() -> Self { Self {} }

    #[inline]
    fn write_mask(&self) -> Mask { Mask::Skip }
}

impl RoleHelper for Server {
    const SHORT_FRAME_HEAD_LEN: u8 = 2 + 4;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2 + 4;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8 + 4;

    #[inline]
    fn new() -> Self { Self {} }

    /// Server should not mask the payload.
    #[inline]
    fn write_mask(&self) -> Mask { Mask::None }
}

impl RoleHelper for StandardClient {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;

    #[inline]
    fn new() -> Self { Self(crate::frame::mask::new_mask_key()) }

    #[inline]
    fn write_mask(&self) -> Mask { Mask::Key(self.0) }

    #[inline]
    fn set_write_mask(&mut self, mask: [u8; 4]) { self.0 = mask; }
}

impl ClientRole for Client {}
impl ServerRole for Server {}
impl ClientRole for StandardClient {}
