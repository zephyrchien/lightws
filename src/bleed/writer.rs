use std::marker::PhantomData;
use std::ptr::copy_nonoverlapping;

pub struct Writer<'a, T> {
    ptr: *mut T,
    pos: usize,
    cap: usize,
    _marker: PhantomData<&'a T>,
}

#[allow(unused)]
#[allow(non_camel_case_types)]
#[allow(clippy::builtin_type_shadow)]
impl<'a, u8> Writer<'a, u8> {
    #[inline]
    pub const fn new(w: &mut [u8]) -> Self {
        Writer {
            ptr: w.as_mut_ptr(),
            pos: 0,
            cap: w.len(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const unsafe fn new_raw(w: *mut u8, pos: usize, cap: usize) -> Self {
        Writer {
            ptr: w,
            pos,
            cap,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn pos(&self) -> usize { self.pos }

    #[inline]
    pub const fn cap(&self) -> usize { self.cap }

    #[inline]
    pub const fn remaining(&self) -> usize { self.cap - self.pos }

    #[inline]
    pub unsafe fn write_unchecked(&mut self, src: &[u8]) -> usize {
        let len = src.len();
        copy_nonoverlapping(src.as_ptr(), self.ptr.add(self.pos), len);
        self.pos += len;
        len
    }

    #[inline]
    pub unsafe fn write_byte_unchecked(&mut self, b: u8) {
        *self.ptr.add(self.pos) = b;
        self.pos += 1;
    }

    #[inline]
    pub fn write_or_err<F, E>(&mut self, src: &[u8], f: F) -> Result<usize, E>
    where
        F: Fn() -> E,
        E: std::error::Error,
    {
        if self.remaining() < src.len() {
            Err(f())
        } else {
            Ok(unsafe { self.write_unchecked(src) })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;

    #[test]
    fn unsafe_write() {
        let mut buf = vec![0; 4096];
        let mut buf2 = buf.clone();

        for i in (1..=1024).filter(|x| 4096 % x == 0) {
            let n = 4096 / i;
            let data: Vec<u8> = std::iter::repeat(rand::random::<u8>()).take(i).collect();

            let mut writer = Writer::new(&mut buf);
            let mut write_n = 0;

            for _ in 0..n {
                unsafe { writer.write_unchecked(&data[..]) };
                {
                    let mut writer2 = &mut buf2.as_mut_slice()[write_n..];
                    write_n += writer2.write(&data[..]).unwrap();
                }
                assert_eq!(write_n, writer.pos());
                assert_eq!(&buf, &buf2);
            }
        }
    }

    #[test]
    fn unsafe_write_byte() {
        let mut buf = vec![0; 4096];
        let mut buf2 = buf.clone();

        let mut writer = Writer::new(&mut buf);
        let mut writer2 = Writer::new(&mut buf2);

        for _ in 0..4096 {
            let b: u8 = rand::random();
            unsafe {
                writer.write_byte_unchecked(b);
                writer2.write_unchecked(&[b]);
                assert_eq!(&buf, &buf2);
            }
        }
    }
}
