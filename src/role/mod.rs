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
pub trait RoleHelper {
    const SHORT_FRAME_HEAD_LEN: u8;
    const COMMON_FRAME_HEAD_LEN: u8;
    const LONG_FRAME_HEAD_LEN: u8;

    fn new_write_mask() -> Mask;
}

/// Client marker.
pub trait ClientRole: RoleHelper {}

/// Server marker.
pub trait ServerRole: RoleHelper {}

/// Simple client using empty(fake) mask key.
///
/// With an empty mask key, the sender/receiver
/// does not need to mask/unmask the payload.
pub struct Client;

/// Standard server.
pub struct Server;

/// Standard client using random mask key.
pub struct StandardClient;

impl RoleHelper for Client {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;

    #[inline]
    fn new_write_mask() -> Mask { Mask::Skip }
}

impl RoleHelper for Server {
    const SHORT_FRAME_HEAD_LEN: u8 = 2 + 4;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2 + 4;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8 + 4;

    /// Server should not mask the payload.
    #[inline]
    fn new_write_mask() -> Mask { Mask::None }
}

impl RoleHelper for StandardClient {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;

    #[inline]
    fn new_write_mask() -> Mask { Mask::Key(crate::frame::mask::new_mask_key()) }
}

impl ClientRole for Client {}
impl ServerRole for Server {}
impl ClientRole for StandardClient {}
