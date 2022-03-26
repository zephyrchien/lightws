use super::Stream;
use super::RoleHelper;
use super::state::{ReadState, WriteState, HeadStore};
use crate::frame::{Fin, OpCode, PayloadLen};
use crate::frame::FrameHead;

#[inline]
pub(super) fn min_len(buf_len: usize, length: u64) -> usize {
    #[cfg(target_pointer_width = "64")]
    {
        std::cmp::min(buf_len, length as usize)
    }

    #[cfg(not(target_pointer_width = "64"))]
    {
        let next = std::cmp::min(usize::MAX as u64, length) as usize;
        std::cmp::min(buf_len, length)
    }
}

#[inline]
pub(super) fn write_data_frame<Role>(store: &mut HeadStore, len: u64)
where
    Role: RoleHelper,
{
    let head = FrameHead::new(
        Fin::Y,
        OpCode::Binary,
        Role::new_write_mask(),
        PayloadLen::from_num(len),
    );
    // The buffer is large enough to accommodate any kind of frame head.
    let n = unsafe { head.encode_unchecked(store.as_mut()) };
    store.set_wr_pos(n);
}

impl<IO, Role> Stream<IO, Role> {
    /// Check if a `Ping` frame is received.
    #[inline]
    pub const fn is_pinged(&self) -> bool { !self.heartbeat.ping_store.is_empty() }

    /// Check if a `Ping` frame is completely read.
    #[inline]
    pub const fn is_ping_completed(&self) -> bool { self.heartbeat.is_complete }

    /// Get the most recent ping. 
    #[inline]
    pub const fn ping_data(&self) -> &[u8] { self.heartbeat.ping_store.read() }

    /// Check if `EOF` is reached.
    #[inline]
    pub const fn is_read_eof(&self) -> bool { matches!(&self.read_state, ReadState::Eof) }

    /// Check if a `Close` frame is received.
    #[inline]
    pub const fn is_read_close(&self) -> bool { matches!(&self.read_state, ReadState::Close) }

    /// Check if a `Close` frame is received or `EOF` is reached.
    #[inline]
    pub const fn is_read_end(&self) -> bool { self.is_read_eof() || self.is_read_close() }

    /// Check if a `WriteZero` error occurred.
    #[inline]
    pub const fn is_write_zero(&self) -> bool { matches!(&self.write_state, WriteState::WriteZero) }

    /// Check if a frame head is partially read.
    #[inline]
    pub const fn is_read_partial_head(&self) -> bool {
        matches!(&self.read_state, ReadState::ReadHead(..))
    }

    /// Check if frame head is partially written.
    #[inline]
    pub const fn is_write_partial_head(&self) -> bool {
        matches!(&self.write_state, WriteState::WriteHead(..))
    }
}
