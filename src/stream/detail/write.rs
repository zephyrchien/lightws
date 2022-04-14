use std::io::Result;
use std::io::IoSlice;
use std::task::{Poll, ready};
use std::marker::PhantomData;

use super::min_len;
use super::super::{Stream, RoleHelper};
use super::super::state::{WriteState, HeadStore};

use crate::frame::FrameHead;
use crate::frame::{Fin, OpCode, PayloadLen};

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
                // build frame head
                // mask payload(this is unsafe) if unsafe_auto_mask_write is activated
                WriteFrameHead::<Role>::write_data_frame(&mut head_store, &mut stream.role, buf);
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

struct WriteFrameHead<Role: RoleHelper> {
    _marker: PhantomData<Role>,
}

trait WriteFrameHeadTrait<R> {
    fn write_data_frame(_: &mut HeadStore, _: &mut R, _: &[u8]) {}
}

// use default impl
impl<Role: RoleHelper> WriteFrameHeadTrait<Role> for WriteFrameHead<Role> {
    #[inline]
    default fn write_data_frame(store: &mut HeadStore, role: &mut Role, buf: &[u8]) {
        let head = FrameHead::new(
            Fin::Y,
            OpCode::Binary,
            role.write_mask_key(),
            PayloadLen::from_num(buf.len() as u64),
        );
        // The buffer is large enough to accommodate any kind of frame head.
        let n = unsafe { head.encode_unchecked(store.as_mut()) };
        store.set_wr_pos(n);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "unsafe_auto_mask_write")] {
        use crate::role::AutoMaskClientRole;
        use crate::bleed::const_cast;
        use crate::frame::{Mask, new_mask_key, apply_mask4};
    }
}

// specialize
#[cfg(feature = "unsafe_auto_mask_write")]
impl<Role: AutoMaskClientRole> WriteFrameHeadTrait<Role> for WriteFrameHead<Role> {
    #[inline]
    fn write_data_frame(store: &mut HeadStore, role: &mut Role, buf: &[u8]) {
        let key = if Role::UPDATE_MASK_KEY {
            let key = new_mask_key();
            role.set_write_mask_key(key);
            key
        } else {
            role.write_mask_key().to_key()
        };

        // !! const_cast a immutable reference
        unsafe {
            let buf = const_cast(buf);
            apply_mask4(key, buf);
        }

        // below is the same of default impl
        let head = FrameHead::new(
            Fin::Y,
            OpCode::Binary,
            Mask::Key(key),
            PayloadLen::from_num(buf.len() as u64),
        );
        // The buffer is large enough to accommodate any kind of frame head.
        let n = unsafe { head.encode_unchecked(store.as_mut()) };
        store.set_wr_pos(n);
    }
}

#[cfg(all(test, feature = "unsafe_auto_mask_write"))]
mod test {
    use super::*;
    use crate::bleed::Store;
    use crate::frame::mask::*;
    use crate::role::*;

    fn auto_mask<R: RoleHelper>(role: &mut R, buf: &[u8]) {
        let mut store = Store::new();
        WriteFrameHead::<R>::write_data_frame(&mut store, role, buf)
    }

    #[test]
    fn auto_mask_active() {
        for i in 0..4096 {
            let mut buf: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(i).collect();
            let buf2 = buf.clone();
            assert_eq!(buf.len(), i);

            let mut role = StandardClient::new();

            for _ in 0..8 {
                auto_mask(&mut role, &buf2);
                let key = role.write_mask_key().to_key();
                apply_mask4(key, &mut buf);
                assert_eq!(buf, buf2);
            }
        }
    }

    #[test]
    fn auto_mask_active2() {
        for i in 0..4096 {
            let mut buf: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(i).collect();
            let buf2 = buf.clone();
            assert_eq!(buf.len(), i);

            let mut role = FixedMaskClient::new();
            let key = role.write_mask_key().to_key();

            for _ in 0..8 {
                auto_mask(&mut role, &buf2);
                assert_eq!(key, role.write_mask_key().to_key());

                apply_mask4(key, &mut buf);
                assert_eq!(buf, buf2);
            }
        }
    }

    #[test]
    fn auto_mask_inactive() {
        for i in 0..4096 {
            let buf: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(i).collect();
            let buf2 = buf.clone();
            assert_eq!(buf.len(), i);

            let mut client = Client::new();
            let mut server = Server::new();

            for _ in 0..8 {
                auto_mask(&mut client, &buf2);
                assert_eq!(buf, buf2);
            }

            for _ in 0..8 {
                auto_mask(&mut server, &buf2);
                assert_eq!(buf, buf2);
            }
        }
    }
}
