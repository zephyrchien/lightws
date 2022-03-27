// #![warn(missing_docs)]
#![feature(const_ptr_offset)]
#![feature(const_slice_from_raw_parts)]
#![feature(const_mut_refs)]
#![feature(const_slice_index)]
#![feature(try_trait_v2)]
#![feature(read_buf)]

//! Lightweight websocket implement for proxy tools.
//! 
//! ## Features
//! - Avoid heap allocation.
//! - Avoid buffering frame payload.
//! - Use vectored-io if available.
//! - Transparent Read/Write over the underlying IO source.
//! 
//! ## High-level API
//! 
//! Stream:
//! 
//! ```ignore
//! {
//!     // establish connection, handshake
//!     let stream = ...
//!     // read some data
//!     stream.read(&mut buf);
//!     // write some data
//!     stream.write(&buf);
//! }
//! ```
//! 
//! ## Low-level API
//! 
//! FrameHead(Fin, OpCode, Mask, PayloadLen):
//! 
//! ```ignore
//! {
//!     // encode a frame head
//!     let head = FrameHead::new(...);
//!     let offset = unsafe {
//!         head.encode_unchecked(&mut buf);
//!     }
//! 
//!     // decode a frame head
//!     let (head, offset) = FrameHead::decode(&buf).unwrap();
//! }
//! ```
//! 
//! Handshake:
//! 
//! ```ignore
//! {
//!     // make a client handshake request
//!     let mut custom_headers = HttpHeader::new_storage();
//!     let request = Request::new(&mut custom_headers);
//!     let offset = request.encode(&mut buf).unwrap();
//! 
//!     // parse a server handshake response
//!     let mut custom_headers = HttpHeader::new_storage();
//!     let mut response = Response::new(&mut custom_headers);
//!     let offset = response.decode(&buf).unwrap();
//! }
//! ```

mod bleed;

pub mod error;
pub mod frame;
pub mod stream;
pub mod handshake;
