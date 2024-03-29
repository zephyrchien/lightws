use std::io::Result;
use std::task::{Poll, ready};

use super::min_len;
use super::super::{Stream, RoleHelper};
use super::super::state::{ReadState, HeadStore};

use crate::frame::{FrameHead, Mask, OpCode};
use crate::frame::mask::apply_mask4;
use crate::error::FrameError;

pub fn read_some<F, IO, Role, Guard>(
    stream: &mut Stream<IO, Role, Guard>,
    mut read: F,
    buf: &mut [u8],
) -> Poll<Result<usize>>
where
    F: FnMut(&mut IO, &mut [u8]) -> Poll<Result<usize>>,
    Role: RoleHelper,
{
    debug_assert!(buf.len() >= 14);

    loop {
        match stream.read_state {
            // always returns 0
            ReadState::Eof => return Poll::Ready(Ok(0)),
            ReadState::Close => return Poll::Ready(Ok(0)),
            // read a new incoming frame
            ReadState::ReadHead(head_store) => {
                let head_store_len = head_store.rd_left();

                // write stored data to user provided buffer
                if !head_store.is_empty() {
                    let (left, _) = buf.split_at_mut(head_store_len);
                    left.copy_from_slice(head_store.read());
                }

                let read_n = ready!(read(&mut stream.io, &mut buf[head_store_len..]))?;

                // EOF ?
                if read_n == 0 {
                    stream.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(0));
                }

                stream.read_state = ReadState::ProcessBuf {
                    beg: 0,
                    end: read_n + head_store_len,
                    processed: 0,
                }
            }
            // continue to read data from the same frame
            ReadState::ReadData { next, mask } => {
                let read_n = ready!(read(&mut stream.io, buf))?;
                // EOF ?
                if read_n == 0 {
                    stream.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(0));
                }
                let len = min_len(read_n, next);
                // unmask if server receives data from client
                // this operation can be skipped if mask key is 0
                if let Mask::Key(key) = mask {
                    apply_mask4(key, &mut buf[..len])
                };
                // read complete ?
                if next > read_n as u64 {
                    // need to read more
                    stream.read_state = ReadState::ReadData {
                        next: next - read_n as u64,
                        mask,
                    };
                    return Poll::Ready(Ok(read_n));
                } else {
                    // continue to process
                    stream.read_state = ReadState::ProcessBuf {
                        beg: len,
                        end: read_n,
                        processed: len,
                    }
                }
            }
            // continue to read data from a ctrl frame
            ReadState::ReadPing { next, mask } => {
                let (buf, _) = stream
                    .heartbeat
                    .ping_store
                    .write()
                    .split_at_mut(next as usize);
                let read_n = ready!(read(&mut stream.io, buf))?;
                // EOF ?
                if read_n == 0 {
                    stream.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(0));
                }
                // unmask if server receives data from client
                // this operation can be skipped if mask key is 0
                if let Mask::Key(key) = mask {
                    apply_mask4(key, buf);
                };

                stream.heartbeat.ping_store.advance_wr_pos(read_n);

                // read complete ?
                if next == read_n as u8 {
                    stream.heartbeat.is_complete = true;
                    stream.read_state = ReadState::new();
                } else {
                    stream.read_state = ReadState::ReadPing {
                        next: next - read_n as u8,
                        mask,
                    };
                }
                return Poll::Ready(Ok(0));
            }
            // handle the read data in user provided buffer
            ReadState::ProcessBuf {
                mut beg,
                end,
                mut processed,
            } => {
                // parse head, fin is ignored
                let (
                    FrameHead {
                        opcode,
                        mask,
                        length,
                        ..
                    },
                    parse_n,
                ) = match FrameHead::decode(&buf[beg..end]) {
                    Ok(x) => x,
                    Err(ref e) if *e == FrameError::NotEnoughData => {
                        if beg == end {
                            stream.read_state = ReadState::new();
                        } else {
                            stream.read_state =
                                ReadState::ReadHead(HeadStore::new_with_data(&buf[beg..end]));
                        }
                        return Poll::Ready(Ok(processed));
                    }
                    Err(e) => return Poll::Ready(Err(e.into())),
                };
                // point to payload
                beg += parse_n;

                // may read a frame without payload
                let frame_len = length.to_num();
                let buf_len = end - beg;
                let data_len = min_len(buf_len, frame_len);

                match opcode {
                    // text is not allowed
                    // we never send a ping, so we ignore the pong
                    OpCode::Text | OpCode::Pong => {
                        return Poll::Ready(Err(FrameError::UnsupportedOpcode.into()));
                    }
                    // ignore fin flag
                    OpCode::Binary | OpCode::Continue => {
                        if data_len != 0 {
                            // unmask payload data from client
                            if let Mask::Key(key) = mask {
                                apply_mask4(key, &mut buf[beg..beg + data_len]);
                            }
                            // move forward
                            unsafe {
                                std::ptr::copy(
                                    buf.as_ptr().add(beg),
                                    buf.as_mut_ptr().add(processed),
                                    data_len,
                                );
                            };
                        }
                        beg += data_len;
                        processed += data_len;
                        // need to read more payload
                        if frame_len > buf_len as u64 {
                            stream.read_state = ReadState::ReadData {
                                next: frame_len - data_len as u64,
                                mask,
                            };
                            return Poll::Ready(Ok(processed));
                        }
                        // continue to process
                        stream.read_state = ReadState::ProcessBuf {
                            beg,
                            end,
                            processed,
                        };
                    }
                    OpCode::Ping => {
                        // a ping frame must not have extened data
                        if frame_len > 125 {
                            return Poll::Ready(Err(FrameError::IllegalData.into()));
                        }
                        if data_len != 0 {
                            // unmask payload data from client
                            if let Mask::Key(key) = mask {
                                apply_mask4(key, &mut buf[beg..beg + data_len]);
                            }
                            // save ping data
                            stream
                                .heartbeat
                                .ping_store
                                .replace_with_data(&buf[beg..beg + data_len]);
                        } else {
                            // no payload
                            stream.heartbeat.ping_store.reset();
                        }

                        // processed does not increase;
                        beg += data_len;

                        // need to read more payload
                        if frame_len > buf_len as u64 {
                            stream.heartbeat.is_complete = false;
                            stream.read_state = ReadState::ReadPing {
                                next: frame_len as u8 - data_len as u8,
                                mask,
                            };
                            return Poll::Ready(Ok(processed));
                        }
                        // continue to process
                        stream.heartbeat.is_complete = true;
                        stream.read_state = ReadState::ProcessBuf {
                            beg,
                            end,
                            processed,
                        };
                    }
                    OpCode::Close => {
                        stream.read_state = ReadState::Close;
                        return Poll::Ready(Ok(processed));
                    }
                }
            }
        }
    }
}
