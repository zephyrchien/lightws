use super::{slice, slice_mut};

/// Buffer on stack.
#[derive(Debug, Clone, Copy)]
pub struct Store<const N: usize> {
    rd: u8,
    wr: u8,
    buf: [u8; N],
}

#[allow(unused)]
impl<const N: usize> Store<N> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            rd: 0,
            wr: 0,
            buf: [0; N],
        }
    }

    #[inline]
    pub fn new_with_data(data: &[u8]) -> Self {
        let mut buf = [0_u8; N];
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), buf.as_mut_ptr(), data.len());
        }
        Self {
            rd: 0,
            wr: data.len() as u8,
            buf,
        }
    }

    #[inline]
    pub fn replace_with_data(&mut self, data: &[u8]) {
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.buf.as_mut_ptr(), data.len());
        }
        self.rd = 0;
        self.wr = data.len() as u8;
    }

    #[inline]
    pub const fn rd_pos(&self) -> usize { self.rd as usize }

    #[inline]
    pub const fn wr_pos(&self) -> usize { self.wr as usize }

    #[inline]
    pub const fn set_rd_pos(&mut self, n: usize) { self.rd = n as u8 }

    #[inline]
    pub const fn set_wr_pos(&mut self, n: usize) { self.wr = n as u8 }

    #[inline]
    pub const fn advance_rd_pos(&mut self, n: usize) { self.rd += n as u8 }

    #[inline]
    pub const fn advance_wr_pos(&mut self, n: usize) { self.wr += n as u8 }

    #[inline]
    pub const fn rd_left(&self) -> usize { self.wr as usize - self.rd as usize }

    #[inline]
    pub const fn wr_left(&self) -> usize { N - self.wr as usize }

    #[inline]
    pub const fn is_empty(&self) -> bool { self.wr == 0 }

    #[inline]
    pub const fn get(&self, idx: usize) -> u8 { unsafe { *self.buf.get_unchecked(idx) } }

    #[inline]
    pub const fn read(&self) -> &[u8] {
        unsafe { slice(&self.buf, self.rd as usize, self.wr as usize) }
    }

    #[inline]
    pub const fn write(&mut self) -> &mut [u8] {
        unsafe { slice_mut(&mut self.buf, self.wr as usize, N) }
    }

    #[inline]
    pub const fn reset(&mut self) {
        self.rd = 0;
        self.wr = 0;
    }
}

/// Get the whole buffer.
impl<const N: usize> AsRef<[u8]> for Store<N> {
    #[inline]
    fn as_ref(&self) -> &[u8] { &self.buf }
}

/// Get the whole buffer.
impl<const N: usize> AsMut<[u8]> for Store<N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] { &mut self.buf }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unsafe_store() {
        let mut store = Store::<14>::new_with_data(b"Hello, ");
        assert_eq!(store.read(), b"Hello, ");
        store.write().copy_from_slice(b"World!!");
        store.advance_wr_pos(7);
        assert_eq!(store.read(), b"Hello, World!!");
        store.advance_rd_pos(7);
        assert_eq!(store.read(), b"World!!");

        store.reset();
        assert_eq!(store.read(), []);

        store.replace_with_data(b"hello, world!!");
        assert_eq!(store.read(), b"hello, world!!");
        store.advance_rd_pos(7);
        assert_eq!(store.read(), b"world!!");

        store.reset();
        assert_eq!(store.read(), []);
    }
}
