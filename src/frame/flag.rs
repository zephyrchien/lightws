//! Fin flag and opcode.

use crate::error::FrameError;

/// Fin flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fin {
    /// a byte with its leading bit set
    Y = 0x80,

    /// a byte with its leading bit clear
    N = 0x00,
}

/// Frame opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// denotes a continuation frame, 0x00
    Continue = 0x00,
    /// denotes a text frame, 0x01
    Text = 0x01,
    /// denotes a binary frame, 0x02
    Binary = 0x02,

    /// denotes a connection close, 0x08
    Close = 0x08,
    /// denotes a ping, 0x09
    Ping = 0x09,
    /// denotes a pong, 0x0a
    Pong = 0x0a,
}

impl Fin {
    /// Parse from byte.
    #[inline]
    pub const fn from_flag(b: u8) -> Result<Self, FrameError> {
        let fin = match b & 0xf0 {
            0x80 => Fin::Y,
            0x00 => Fin::N,
            _ => return Err(FrameError::IllegalFin),
        };
        Ok(fin)
    }
}

impl OpCode {
    /// Parse from byte.
    #[inline]
    pub const fn from_flag(b: u8) -> Result<Self, FrameError> {
        use OpCode::*;
        let opcode = match b & 0x0f {
            0x00 => Continue,
            0x01 => Text,
            0x02 => Binary,
            0x08 => Close,
            0x09 => Ping,
            0x0a => Pong,
            _ => return Err(FrameError::IllegalOpCode),
        };
        Ok(opcode)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules!  enc_dec {
        ($class: ident $(, $v: expr )+ ) => {
            $(
                let v = $class::from_flag($v).unwrap();
                assert_eq!(v as u8, $v);
            )+
        };
    }

    #[test]
    fn fin() {
        enc_dec!(Fin, 0x00, 0x80);
    }

    #[test]
    fn opcode() {
        enc_dec!(OpCode, 0x00, 0x01, 0x02, 0x08, 0x09, 0x0a);
    }
}
