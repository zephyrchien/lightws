// #![warn(missing_docs)]
#![feature(const_slice_from_raw_parts)]
#![feature(const_mut_refs)]
#![feature(const_slice_index)]
#![feature(read_buf)]
#![feature(ready_macro)]

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
//! - [`role`]
//! - [`endpoint`]
//! - [`stream`]
//!
//! ```ignore
//! {
//!     // handshake
//!     let stream = Endpoint<TcpStream, Client>::connect(tcp, buf, host, path)?;
//!     // read some data
//!     stream.read(&mut buf);
//!     // write some data
//!     stream.write(&buf);
//! }
//! ```
//!
//! ## Low-level API
//!
//! - [`frame`]
//! - [`handshake`]
//!
//! Frame:
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

pub mod role;
pub mod error;
pub mod frame;
pub mod stream;
pub mod endpoint;
pub mod handshake;
