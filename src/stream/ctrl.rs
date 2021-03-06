use super::Stream;
use super::state::WriteState;

use crate::frame::Mask;
use crate::role::RoleHelper;
use crate::error::CtrlError;

impl<IO, Role, Guard> Stream<IO, Role, Guard>
where
    Role: RoleHelper,
{
    /// Get mask for upcoming writes.
    #[inline]
    pub fn mask_key(&self) -> Mask { self.role.mask_key() }

    /// Set mask for upcoming writes.
    /// An attempt to set mask during a write will fail with [`CtrlError::SetMaskInWrite`].
    #[inline]
    pub fn set_mask_key(&mut self, key: [u8; 4]) -> Result<(), CtrlError> {
        // make sure this is a new fresh write
        if let WriteState::WriteHead(head) = self.write_state {
            if head.is_empty() {
                self.role.set_mask_key(key);
                return Ok(());
            }
        }
        Err(CtrlError::SetMaskInWrite)
    }
}
