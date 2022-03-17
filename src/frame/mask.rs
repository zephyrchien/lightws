//!  Mask flag and key.

use crate::error::FrameError;

/// Payload mask with a 32-bit key.
///
/// `Mask::Skip` is used by server side to skip unmask
/// if mask key equals 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    Key([u8; 4]),
    Skip,
    None,
}

impl Mask {
    /// Read the flag which indicates whether mask is used.
    #[inline]
    pub const fn from_flag(b: u8) -> Result<Self, FrameError> {
        let mask = match b & 0x80 {
            0x80 => Mask::Skip,
            0x00 => Mask::None,
            _ => return Err(FrameError::IllegalMask),
        };
        Ok(mask)
    }

    /// Get the flag byte.
    #[inline]
    pub const fn to_flag(&self) -> u8 {
        use Mask::*;
        match self {
            Key(_) | Skip => 0x80,
            None => 0x00,
        }
    }
}

/// Generate a new random key.
#[inline]
pub fn new_rand_key() -> [u8; 4] { rand::random::<[u8; 4]>() }

/// Mask the buffer, byte by byte.
#[inline]
pub fn apply_mask(key: [u8; 4], buf: &mut [u8]) {
    for (i, b) in buf.iter_mut().enumerate() {
        *b ^= key[i & 0x03];
    }
}

/// Mask the buffer, 4 bytes at a time.
#[inline]
pub fn apply_mask4(key: [u8; 4], buf: &mut [u8]) {
    let key4 = u32::from_ne_bytes(key);

    let (prefix, middle, suffix) = unsafe { buf.align_to_mut::<u32>() };

    apply_mask(key, prefix);

    let head = prefix.len() & 3;
    let key4 = if head > 0 {
        if cfg!(target_endian = "big") {
            key4.rotate_left(8 * head as u32)
        } else {
            key4.rotate_right(8 * head as u32)
        }
    } else {
        key4
    };
    for b4 in middle.iter_mut() {
        *b4 ^= key4;
    }

    apply_mask(key4.to_ne_bytes(), suffix);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mask_store() {
        for v in [0x00, 0x80] {
            assert_eq!(Mask::from_flag(v).unwrap().to_flag(), v);
        }
    }

    #[test]
    fn mask_byte() {
        let key: [u8; 4] = rand::random();
        let buf: Vec<u8> =
            std::iter::repeat(rand::random::<u8>()).take(1024).collect();

        assert_eq!(buf.len(), 1024);

        let mut buf2 = buf.clone();
        apply_mask(key, &mut buf2);
        apply_mask(key, &mut buf2);

        assert_eq!(buf, buf2);
    }

    #[test]
    fn mask_byte4() {
        for i in 0..4096 {
            let key: [u8; 4] = rand::random();
            let buf: Vec<u8> =
                std::iter::repeat(rand::random::<u8>()).take(i).collect();

            assert_eq!(buf.len(), i);

            let mut buf2 = buf.clone();
            apply_mask4(key, &mut buf2);
            apply_mask4(key, &mut buf2);

            assert_eq!(buf, buf2);
        }
    }
}
