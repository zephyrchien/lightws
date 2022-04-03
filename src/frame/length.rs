//! Payload length.

/// Payload length.
///
/// Could be 7 bits, 7+16 bits, or 7+64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadLen {
    /// 0 - 125
    Standard(u8),
    /// 126 - 65535
    Extended1(u16),
    /// over 65536
    Extended2(u64),
}

impl PayloadLen {
    /// Parse from number.
    #[inline]
    pub const fn from_num(n: u64) -> Self {
        if n < 126 {
            PayloadLen::Standard(n as u8)
        } else if n < 65536 {
            PayloadLen::Extended1(n as u16)
        } else {
            PayloadLen::Extended2(n)
        }
    }

    /// Convert to number.
    #[inline]
    pub const fn to_num(self) -> u64 {
        use PayloadLen::*;
        match self {
            Standard(v) => v as u64,
            Extended1(v) => v as u64,
            Extended2(v) => v,
        }
    }

    /// Read the flag which indicates the kind of length.
    ///
    /// If extended length is used, the caller should read the next 2 or 8 bytes
    /// to get the real length.
    #[inline]
    pub const fn from_flag(b: u8) -> Self {
        match b & 0x7f {
            126 => PayloadLen::Extended1(0),
            127 => PayloadLen::Extended2(0),
            b => PayloadLen::Standard(b),
        }
    }

    /// Generate the flag byte.
    /// If `length <= 125`, it represents the real length.
    #[inline]
    pub const fn to_flag(&self) -> u8 {
        use PayloadLen::*;
        match self {
            Standard(b) => *b,
            Extended1(_) => 126,
            Extended2(_) => 127,
        }
    }

    /// Read as 16-bit length.
    #[inline]
    pub const fn from_byte2(buf: [u8; 2]) -> Self { PayloadLen::Extended1(u16::from_be_bytes(buf)) }

    /// Read as 64-bit length.
    #[inline]
    pub const fn from_byte8(buf: [u8; 8]) -> Self { PayloadLen::Extended2(u64::from_be_bytes(buf)) }

    /// Get value, as 8-bit length.
    #[inline]
    pub const fn to_u8(&self) -> u8 {
        match self {
            PayloadLen::Standard(v) => *v,
            _ => unreachable!(),
        }
    }

    /// Get value, as 16-bit length.
    #[inline]
    pub const fn to_u16(&self) -> u16 {
        match self {
            PayloadLen::Extended1(v) => *v,
            _ => unreachable!(),
        }
    }

    /// Get value, as 64-bit length.
    #[inline]
    pub const fn to_u64(&self) -> u64 {
        match self {
            PayloadLen::Extended2(v) => *v,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn standard() {
        for v in 0..=125_u8 {
            let a = PayloadLen::from_flag(v);
            let b = PayloadLen::from_num(v as u64);

            assert_eq!(a.to_flag(), v);
            assert_eq!(a.to_num(), b.to_num());
        }
    }

    #[test]
    fn extend1() {
        for v in 126..=65535_u16 {
            let a = PayloadLen::from_num(v as u64);
            let b = PayloadLen::from_byte2(v.to_be_bytes());

            assert_eq!(a.to_flag(), 126_u8);
            assert_eq!(a.to_num(), b.to_num());
        }
    }

    #[test]
    fn extend2() {
        for v in 65536..=100000_u64 {
            let a = PayloadLen::from_num(v);
            let b = PayloadLen::from_byte8(v.to_be_bytes());

            assert_eq!(a.to_flag(), 127_u8);
            assert_eq!(a.to_num(), b.to_num());
        }
    }
}
