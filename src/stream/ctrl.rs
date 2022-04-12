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
    pub fn write_mask(&self) -> Mask { self.role.write_mask() }

    /// Set mask for upcoming writes.
    /// An attempt to set mask during a write will fail with [`CtrlError::SetMaskInWrite`].
    #[inline]
    pub fn set_write_mask(&mut self, mask: [u8; 4]) -> Result<(), CtrlError> {
        // make sure this is a new fresh write
        if let WriteState::WriteHead(head) = self.write_state {
            if head.is_empty() {
                self.role.set_write_mask(mask);
                return Ok(());
            }
        }
        Err(CtrlError::SetMaskInWrite)
    }
}
