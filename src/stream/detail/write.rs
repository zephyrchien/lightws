use std::io::Result;
use std::io::IoSlice;
use std::task::{Poll, ready};

use super::min_len;
use super::super::{Stream, RoleHelper};
use super::super::state::{WriteState, HeadStore};

use crate::frame::FrameHead;
use crate::frame::{Fin, OpCode, PayloadLen};

#[inline]
fn write_data_frame<Role>(store: &mut HeadStore, len: u64)
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

pub fn write_some<F, IO, Role, Guard>(
    mut stream: &mut Stream<IO, Role, Guard>,
    mut write: F,
    buf: &[u8],
) -> Poll<Result<usize>>
where
    F: FnMut(&mut IO, &[IoSlice]) -> Poll<Result<usize>>,
    Role: RoleHelper,
{
    match stream.write_state {
        // always returns 0
        WriteState::WriteZero => Poll::Ready(Ok(0)),
        // create a new frame
        WriteState::WriteHead(mut head_store) => {
            // data frame length depends on provided buffer length
            let frame_len = buf.len();

            if head_store.is_empty() {
                write_data_frame::<Role>(&mut head_store, frame_len as u64);
            }
            // frame head(maybe partial) + payload
            let iovec = [IoSlice::new(head_store.read()), IoSlice::new(buf)];
            let write_n = ready!(write(&mut stream.io, &iovec))?;
            let head_len = head_store.rd_left() as usize;

            // write zero ?
            if write_n == 0 {
                stream.write_state = WriteState::WriteZero;
                return Poll::Ready(Ok(0));
            }

            // frame head is not written completely
            if write_n < head_len {
                head_store.advance_rd_pos(write_n);
                stream.write_state = WriteState::WriteHead(head_store);
                return Poll::Ready(Ok(0));
            }

            // frame has been written completely
            let write_n = write_n - head_len;

            // all data written ?
            if write_n == frame_len {
                stream.write_state = WriteState::new();
            } else {
                stream.write_state = WriteState::WriteData((frame_len - write_n) as u64);
            }

            Poll::Ready(Ok(write_n))
        }
        // continue to write to the same frame
        WriteState::WriteData(next) => {
            let len = min_len(buf.len(), next);
            let write_n = ready!(write(&mut stream.io, &[IoSlice::new(&buf[..len])]))?;
            // write zero ?
            if write_n == 0 {
                stream.write_state = WriteState::WriteZero;
                return Poll::Ready(Ok(0));
            }
            // all data written ?
            if next == write_n as u64 {
                stream.write_state = WriteState::new()
            } else {
                stream.write_state = WriteState::WriteData(next - write_n as u64)
            }
            Poll::Ready(Ok(write_n))
        }
    }
}
