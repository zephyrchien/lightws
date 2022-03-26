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
