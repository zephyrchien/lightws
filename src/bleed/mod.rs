//! Some dirty work

mod writer;

pub(crate) use writer::Writer;

#[inline]
pub(crate) const unsafe fn slice<T>(
    slice: &[T],
    beg: usize,
    end: usize,
) -> &[T] {
    let ptr = slice.as_ptr().add(beg);
    &*std::ptr::slice_from_raw_parts(ptr, end - beg)
}

#[inline]
pub(crate) const unsafe fn slice_to_array<T, const N: usize>(
    slice: &[T],
) -> &[T; N] {
    &*(slice as *const [T] as *const [T; N])
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unsafe_slice() {
        let buf: Vec<u8> =
            std::iter::repeat(rand::random::<u8>()).take(1024).collect();
        macro_rules! s {
            ($beg: expr, $end: expr) => {
                assert_eq!(&buf[$beg..$end], unsafe {
                    slice(&buf, $beg, $end)
                });
            };
        }
        s!(0, 1024);
        s!(1, 123);
        s!(2, 333);
        s!(3, 444);
        s!(555, 666);
        s!(777, 888);
    }

    #[test]
    fn unsafe_slice_to_array() {
        let buf: Vec<u8> =
            std::iter::repeat(rand::random::<u8>()).take(4096).collect();

        macro_rules! s {
            ($beg: expr, $len: expr) => {
                let slice = &buf[$beg..$beg + $len];
                let array1: [_; $len] = slice.try_into().unwrap();
                let array2: [_; $len] =
                    *unsafe { slice_to_array::<_, $len>(slice) };

                assert_eq!(array1, array2);
            };
        }

        s!(0, 1024);
        s!(1, 123);
        s!(2, 333);
        s!(3, 444);
        s!(555, 666);
        s!(777, 888);
    }
}
