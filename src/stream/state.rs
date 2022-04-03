use super::Stream;

use crate::frame::Mask;
use crate::bleed::Store;

/// Store incomplete frame head.
pub(super) type HeadStore = Store<14>;

/// Store the most recent ping.
pub(super) type PingStore = Store<125>;

#[derive(Debug)]
pub(super) struct HeartBeat {
    pub ping_store: PingStore,
    pub is_complete: bool,
}

impl HeartBeat {
    #[inline]
    pub const fn new() -> Self {
        Self {
            ping_store: PingStore::new(),
            is_complete: false,
        }
    }
}

/// Read state.
#[derive(Debug)]
pub(super) enum ReadState {
    ReadHead(HeadStore),
    ReadData {
        next: u64,
        mask: Mask,
    },
    ReadPing {
        next: u8,
        mask: Mask,
    },
    ProcessBuf {
        beg: usize,
        end: usize,
        processed: usize,
    },
    Eof,
    Close,
}

impl ReadState {
    #[inline]
    pub const fn new() -> Self { ReadState::ReadHead(Store::new()) }
}

/// Write state.
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub(super) enum WriteState {
    WriteHead(HeadStore),
    WriteData(u64),
    WriteZero,
}

impl WriteState {
    #[inline]
    pub const fn new() -> Self { WriteState::WriteHead(Store::new()) }
}

/// Check status.
impl<IO, Role, Guard> Stream<IO, Role, Guard> {
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
