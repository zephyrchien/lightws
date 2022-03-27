use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    IllegalFin,

    IllegalMask,

    IllegalOpCode,

    IllegalData,

    NotEnoughData,

    NotEnoughCapacity,

    UnsupportedOpcode,
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FrameError::*;
        match self {
            IllegalFin => write!(f, "Illegal fin value"),
            IllegalMask => write!(f, "Illegal mask value"),
            IllegalOpCode => write!(f, "Illegal opcode value"),
            IllegalData => write!(f, "Illegal data"),
            NotEnoughData => write!(f, "Not enough data to parse"),
            NotEnoughCapacity => write!(f, "Not enough space to write to"),
            UnsupportedOpcode => write!(
                f,
                "Unsupported opcode, only support binary, ping, pong, close"
            ),
        }
    }
}

// use default impl
impl std::error::Error for FrameError {}
