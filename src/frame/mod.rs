//! Websocket data frame.
//!
//! [RFC-6455 Section5](https://datatracker.ietf.org/doc/html/rfc6455#section-5)
//!
//! ```text
//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-------+-+-------------+-------------------------------+
//! |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
//! |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
//! |N|V|V|V|       |S|             |   (if payload len==126/127)   |
//! | |1|2|3|       |K|             |                               |
//! +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
//! |     Extended payload length continued, if payload len == 127  |
//! + - - - - - - - - - - - - - - - +-------------------------------+
//! |                               |Masking-key, if MASK set to 1  |
//! +-------------------------------+-------------------------------+
//! | Masking-key (continued)       |          Payload Data         |
//! +-------------------------------- - - - - - - - - - - - - - - - +
//! :                     Payload Data continued ...                :
//! + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
//! |                     Payload Data continued ...                |
//! +---------------------------------------------------------------+
//! ```
//!

pub mod flag;
pub mod length;
pub mod mask;

pub use flag::{Fin, OpCode};
pub use length::PayloadLen;
pub use mask::Mask;

/// Websocket frame head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHead {
    pub fin: Fin,
    pub opcode: OpCode,
    pub mask: Mask,
    pub length: PayloadLen,
}

use crate::bleed::Writer;
use crate::bleed::{slice, slice_to_array};
use crate::error::FrameError;

impl FrameHead {
    /// Constructor.
    #[inline]
    pub const fn new(fin: Fin, opcode: OpCode, mask: Mask, length: PayloadLen) -> Self {
        Self {
            fin,
            opcode,
            mask,
            length,
        }
    }

    /// Encode to provided buffer, returns the count of written bytes.
    /// The caller should ensure the buffer is large enough,
    /// otherwise a [`FrameError::NotEnoughCapacity`] error will be returned.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, FrameError> {
        if buf.len() < 2 {
            return Err(FrameError::NotEnoughCapacity);
        }

        let mut writer = Writer::new(buf);

        macro_rules! writex {
            ($dst: expr) => {
                if writer.remaining() < $dst.len() {
                    return Err(FrameError::NotEnoughCapacity);
                } else {
                    unsafe {
                        writer.write_unchecked($dst);
                    }
                }
            };
        }

        // fin, opcode
        let b1 = self.fin as u8 | self.opcode as u8;

        // mask, payload length
        let b2 = self.mask.to_flag() | self.length.to_flag();

        writex!(&[b1, b2]);

        // extended payload length
        match &self.length {
            PayloadLen::Standard(_) => {}
            PayloadLen::Extended1(v) => writex!(&v.to_be_bytes()),
            PayloadLen::Extended2(v) => writex!(&v.to_be_bytes()),
        };

        // mask key
        match &self.mask {
            Mask::Key(k) => writex!(k),
            Mask::Skip => writex!(&[0u8; 4]),
            Mask::None => {}
        };

        Ok(writer.pos())
    }

    /// Unchecked version of [`encode`](Self::encode).
    ///
    /// # Safety
    ///
    /// Caller must ensure buffer is large enough. It is **Undefined Behavior** if the
    /// buffer is not large enough.
    pub unsafe fn encode_unchecked(&self, buf: &mut [u8]) -> usize {
        let mut writer = Writer::new(buf);

        macro_rules! writex {
            ($dst: expr) => {{
                writer.write_unchecked($dst);
            }};
        }

        // fin, opcode
        let b1 = self.fin as u8 | self.opcode as u8;

        // mask, payload length
        let b2 = self.mask.to_flag() | self.length.to_flag();

        writex!(&[b1, b2]);

        // extended payload length
        match &self.length {
            PayloadLen::Standard(_) => {}
            PayloadLen::Extended1(v) => writex!(&v.to_be_bytes()),
            PayloadLen::Extended2(v) => writex!(&v.to_be_bytes()),
        };

        // mask key
        match &self.mask {
            Mask::Key(k) => writex!(k),
            Mask::Skip => writex!(&[0u8; 4]),
            Mask::None => {}
        };

        writer.pos()
    }

    /// Parse from provided buffer, returns [`FrameHead`] and the count of read bytes
    /// if the parse succeeds.
    /// If there is not enough data to parse, a [`FrameError::NotEnoughData`] error
    /// will be returned.
    pub fn decode(buf: &[u8]) -> Result<(Self, usize), FrameError> {
        if buf.len() < 2 {
            return Err(FrameError::NotEnoughData);
        }

        let mut n: usize = 2;

        // fin, opcode
        let b1 = unsafe { *buf.get_unchecked(0) };

        // mask, payload length
        let b2 = unsafe { *buf.get_unchecked(1) };

        let fin = Fin::from_flag(b1)?;
        let opcode = OpCode::from_flag(b1)?;

        let mut mask = Mask::from_flag(b2)?;
        let mut length = PayloadLen::from_flag(b2);

        match length {
            PayloadLen::Standard(_) => {}
            PayloadLen::Extended1(_) => {
                if buf.len() - n < 2 {
                    return Err(FrameError::NotEnoughData);
                }

                length =
                    PayloadLen::from_byte2(unsafe { *slice_to_array::<_, 2>(slice(buf, 2, 4)) });

                n += 2;
            }
            PayloadLen::Extended2(_) => {
                if buf.len() - n < 8 {
                    return Err(FrameError::NotEnoughData);
                }

                length =
                    PayloadLen::from_byte8(unsafe { *slice_to_array::<_, 8>(slice(buf, 2, 10)) });

                n += 8;
            }
        };

        match mask {
            Mask::None => {}
            _ => {
                if buf.len() - n < 4 {
                    return Err(FrameError::NotEnoughData);
                }

                let key = *unsafe { slice_to_array::<_, 4>(slice(buf, n, n + 4)) };

                if key.into_iter().all(|b| b == 0) {
                    mask = Mask::Skip
                } else {
                    mask = Mask::Key(key)
                }

                n += 4;
            }
        }

        Ok((
            FrameHead {
                fin,
                opcode,
                mask,
                length,
            },
            n,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn frame_head() {
        let head = FrameHead {
            fin: Fin::Y,
            opcode: OpCode::Binary,
            mask: Mask::Key(mask::new_rand_key()),
            length: PayloadLen::from_num(4096),
        };

        let head2 = FrameHead {
            fin: Fin::N,
            opcode: OpCode::Binary,
            mask: Mask::Key(mask::new_rand_key()),
            length: PayloadLen::from_num(64),
        };

        for head in [head, head2] {
            let mut buf = vec![0; 1024];

            let encode_n = head.encode(&mut buf).unwrap();

            assert!(encode_n + 128 <= buf.len());

            let (head2, decode_n) = FrameHead::decode(&buf[0..encode_n + 128]).unwrap();

            assert_eq!(encode_n, decode_n);
            assert_eq!(head, head2);

            let mut buf2 = vec![0; 1024];
            let encode_n2 = unsafe { head2.encode_unchecked(&mut buf2) };

            assert_eq!(encode_n2, encode_n);
            assert_eq!(&buf[0..encode_n], &buf2[0..encode_n2]);
        }
    }
}
