//! Some dirty work

mod store;
mod writer;

pub(crate) use store::Store;
pub(crate) use writer::Writer;

#[inline]
pub(crate) const unsafe fn slice<T>(slice: &[T], beg: usize, end: usize) -> &[T] {
    let ptr = slice.as_ptr().add(beg);
    &*std::ptr::slice_from_raw_parts(ptr, end - beg)
}

#[inline]
pub(crate) const unsafe fn slice_mut<T>(slice: &mut [T], beg: usize, end: usize) -> &mut [T] {
    let ptr = slice.as_mut_ptr().add(beg);
    &mut *std::ptr::slice_from_raw_parts_mut(ptr, end - beg)
}

#[inline]
pub(crate) const unsafe fn slice_to_array<T, const N: usize>(slice: &[T]) -> &[T; N] {
    &*(slice as *const [T] as *const [T; N])
}

#[inline]
#[allow(unused)]
#[allow(invalid_reference_casting)]
#[allow(clippy::mut_from_ref)]
pub(crate) const unsafe fn const_cast<T: ?Sized>(x: &T) -> &mut T {
    let const_ptr = x as *const T;
    let mut_ptr = const_ptr.cast_mut();
    &mut *mut_ptr
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unsafe_slice() {
        let buf: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(1024).collect();
        let mut buf2 = buf.clone();

        macro_rules! s {
            ($beg: expr, $end: expr) => {
                assert_eq!(&buf[$beg..$end], unsafe { slice(&buf, $beg, $end) });
                assert_eq!(&buf[$beg..$end], unsafe {
                    slice_mut(&mut buf2, $beg, $end)
                });
            };
        }

        for end in 1..1024 {
            for beg in 0..end {
                s!(beg, end);
            }
        }
    }

    #[test]
    fn unsafe_slice_to_array() {
        let buf: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(4096).collect();

        macro_rules! s {
            ($beg: expr, $len: expr) => {
                let slice = &buf[$beg..$beg + $len];
                let array1: [_; $len] = slice.try_into().unwrap();
                let array2: [_; $len] = *unsafe { slice_to_array::<_, $len>(slice) };

                assert_eq!(array1, array2);
            };
        }

        for beg in 0..=2048 {
            s!(beg, 2048);
        }
    }
}
