use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum CtrlError {
    SetMaskInWrite,
}

impl Display for CtrlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CtrlError::*;
        match self {
            SetMaskInWrite => write!(f, "Set mask during an incomplete write"),
        }
    }
}

// use default impl
impl std::error::Error for CtrlError {}
