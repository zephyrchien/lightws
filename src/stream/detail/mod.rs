mod read;
mod write;

pub(super) use read::read_some;
pub(super) use write::write_some;

#[inline]
fn min_len(buf_len: usize, length: u64) -> usize {
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
