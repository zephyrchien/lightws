use super::{RoleHelper, ServerRole};
use crate::frame::Mask;

/// Standard server.
#[derive(Clone, Copy)]
pub struct Server;

impl RoleHelper for Server {
    const SHORT_FRAME_HEAD_LEN: u8 = 2 + 4;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2 + 4;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8 + 4;

    #[inline]
    fn new() -> Self { Self {} }

    /// Server should not mask the payload.
    #[inline]
    fn write_mask_key(&self) -> Mask { Mask::None }
}

impl ServerRole for Server {}
