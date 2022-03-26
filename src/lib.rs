// #![warn(missing_docs)]
#![feature(const_ptr_offset)]
#![feature(const_slice_from_raw_parts)]
#![feature(const_mut_refs)]
#![feature(const_slice_index)]
#![feature(read_buf)]

//! Lightweight websocket implement for proxy tools.

mod bleed;

pub mod error;
pub mod frame;
pub mod stream;
pub mod handshake;
