use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum FrameError {
    IllegalFin,

    IllegalMask,

    IllegalOpCode,

    NotEnoughData,

    NotEnoughCapacity,
}

impl Display for FrameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FrameError::*;
        match self {
            IllegalFin => write!(f, "Illegal fin value"),
            IllegalMask => write!(f, "Illegal mask value"),
            IllegalOpCode => write!(f, "Illegal opcode value"),
            NotEnoughData => write!(f, "Not enough data to parse"),
            NotEnoughCapacity => write!(f, "Not enough space to write to"),
        }
    }
}

// use default impl
impl std::error::Error for FrameError {}
