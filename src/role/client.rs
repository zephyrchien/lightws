use super::{RoleHelper, ClientRole, AutoMaskClientRole};
use crate::frame::Mask;

macro_rules! client_consts {
    () => {
        const SHORT_FRAME_HEAD_LEN: u8 = 2;
        const COMMON_FRAME_HEAD_LEN: u8 = 2 + 2;
        const LONG_FRAME_HEAD_LEN: u8 = 2 + 8;
    };
}

/// Simple client using an empty(fake) mask key.
///
/// It simply skips masking before writing data.
#[derive(Clone, Copy)]
pub struct Client;

impl RoleHelper for Client {
    client_consts!();

    #[inline]
    fn new() -> Self { Self {} }

    #[inline]
    fn mask_key(&self) -> Mask { Mask::Skip }
}

impl ClientRole for Client {}

/// Standard client using random mask key.
///
/// With `unsafe_auto_mask_write` feature enabled, it will automatically
/// update its inner mask key and mask payload data before a write.
#[derive(Clone, Copy)]
pub struct StandardClient([u8; 4]);

impl RoleHelper for StandardClient {
    client_consts!();

    #[inline]
    fn new() -> Self { Self([9u8; 4]) }

    #[inline]
    fn mask_key(&self) -> Mask { Mask::Key(self.0) }

    #[inline]
    fn set_mask_key(&mut self, mask: [u8; 4]) { self.0 = mask; }
}

impl ClientRole for StandardClient {}

impl AutoMaskClientRole for StandardClient {
    const UPDATE_MASK_KEY: bool = true;
}

/// Client using a fixed mask key.
///
/// With `unsafe_auto_mask_write` feature enabled, it will automatically
/// mask payload data before a write, where its inner mask key is not updated.
#[derive(Clone, Copy)]
pub struct FixedMaskClient([u8; 4]);

impl RoleHelper for FixedMaskClient {
    client_consts!();

    #[inline]
    fn new() -> Self { Self([9u8; 4]) }

    #[inline]
    fn mask_key(&self) -> Mask { Mask::Key(self.0) }

    #[inline]
    fn set_mask_key(&mut self, mask: [u8; 4]) { self.0 = mask; }
}

impl ClientRole for FixedMaskClient {}

impl AutoMaskClientRole for FixedMaskClient {
    const UPDATE_MASK_KEY: bool = false;
}
