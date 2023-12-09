#![allow(incomplete_features)]
#![feature(const_mut_refs)]
#![feature(const_slice_index)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(read_buf)]
#![feature(core_io_borrowed_buf)]
#![feature(specialization)]

//! Lightweight websocket implement for stream transmission.
//!
//! ## Features
//!
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
//! Std:
//!
//! ```no_run
//! use std::io::{Read, Write};
//! use std::net::TcpStream;
//! use lightws::role::Client;
//! use lightws::endpoint::Endpoint;
//! fn run_sync() -> std::io::Result<()> {  
//!     let mut buf = [0u8; 256];
//!     // establish tcp connection
//!     let mut tcp = TcpStream::connect("example.com:80")?;
//!     // establish ws connection
//!     let mut ws = Endpoint::<TcpStream, Client>::connect(tcp, &mut buf, "example.com", "/ws")?;
//!     // read some data
//!     let n = ws.read(&mut buf)?;
//!     // write some data
//!     let n = ws.write(&buf)?;
//!     Ok(())
//! }
//! ```
//!
//! Tokio:
//!
//! ```no_run
//! use tokio::net::TcpStream;
//! use tokio::io::{AsyncReadExt, AsyncWriteExt};
//! use lightws::role::Client;
//! use lightws::endpoint::Endpoint;
//! async fn run_async() -> std::io::Result<()> {
//!     let mut buf = [0u8; 256];
//!     // establish tcp connection
//!     let mut tcp = TcpStream::connect("example.com:80").await?;
//!     // establish ws connection
//!     let mut ws = Endpoint::<TcpStream, Client>::connect_async(tcp, &mut buf, "example.com", "/ws").await?;
//!     // read some data
//!     let n = ws.read(&mut buf).await?;
//!     // write some data
//!     let n = ws.write(&buf).await?;
//!     Ok(())
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
//! ```no_run
//! use lightws::frame::{FrameHead, Fin, OpCode, PayloadLen, Mask};
//! {
//!     let mut buf = [0u8; 14];
//!     // crate a frame head
//!     let head = FrameHead::new(
//!         Fin::N, OpCode::Binary,
//!         Mask::None, PayloadLen::from_num(256)
//!     );
//!     // encode to buffer
//!     let offset = unsafe {
//!         head.encode_unchecked(&mut buf);
//!     };
//!     // decode from buffer
//!     let (head, offset) = FrameHead::decode(&buf).unwrap();
//! }
//! ```
//!
//! Handshake:
//!
//! ```no_run
//! use lightws::handshake::{Request, Response, HttpHeader};
//! {
//!     let mut buf = [0u8; 256];
//!     // make a client handshake request
//!     let request = Request::new(b"/ws", b"example.com", b"sec-key..");
//!     let offset = request.encode(&mut buf).unwrap();
//!
//!     // parse a server handshake response
//!     let mut custom_headers = HttpHeader::new_storage();
//!     let mut response = Response::new_storage(&mut custom_headers);
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
