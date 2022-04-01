//! Websocket client or server.

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

/// Simple Client.
pub struct Client;

/// Simple Server.
pub struct Server;

impl RoleHelper for Client {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;

    /// Client uses a zero mask key, so that the sender/receiver
    /// does not need to mask/unmask the payload.
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

impl ClientRole for Client {}
impl ServerRole for Server {}
