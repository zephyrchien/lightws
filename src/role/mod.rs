//! Markers.
//!
//! Markers are used to apply different strategies to clients or servers.
//!
//! For example, `Endpoint<IO, Client>::connect` is used to to open a connection,
//! and returns `Stream<IO, Client>`; `Endpoint<IO, Server>` is used to accept
//! a connection and returns `Stream<IO, Server>`.
//!
//! Both client and server meet [`RoleHelper`], which indicates frame head length
//! (currently unused), and how to mask payload data. Only client meets [`ClientRole`],
//! and only server meets [`ServerRole`].
//!
//! Any type implements these traits will be treated as a `client` or `server`.

use crate::frame::Mask;

/// Client or Server marker.
pub trait RoleHelper: Clone + Copy {
    const SHORT_FRAME_HEAD_LEN: u8;
    const COMMON_FRAME_HEAD_LEN: u8;
    const LONG_FRAME_HEAD_LEN: u8;

    fn new() -> Self;
    fn write_mask_key(&self) -> Mask;
    // by default this is a no-op
    fn set_write_mask_key(&mut self, _: [u8; 4]) {}
}

/// Client marker.
pub trait ClientRole: RoleHelper {
    const SHORT_FRAME_HEAD_LEN: u8 = 2;
    const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
    const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;
}

/// Server marker.
pub trait ServerRole: RoleHelper {}

/// Client marker.
pub trait AutoMaskClientRole: ClientRole {
    const UPDATE_MASK_KEY: bool;
}

mod server;
mod client;

pub use server::Server;
pub use client::{Client, StandardClient, FixedMaskClient};
