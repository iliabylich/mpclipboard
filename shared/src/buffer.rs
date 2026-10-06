use crate::{NonEmptyInlineString, NonEmptyInlineStringLength};
use core::ops::{Add, ControlFlow};
use generic_array::{ArrayLength, GenericArray};
use typenum::Sum;

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Buffer<N: ArrayLength> {
    buf: GenericArray<u8, N>,
    len: usize,
}

impl<N: ArrayLength> Buffer<N> {
    pub fn empty() -> Self {
        Self {
            buf: GenericArray::default(),
            len: 0,
        }
    }

    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        let mut buf = GenericArray::default();
        buf.get_mut(..bytes.len())?.copy_from_slice(bytes);
        Some(Self {
            buf,
            len: bytes.len(),
        })
    }

    pub const fn new(buf: GenericArray<u8, N>) -> Self {
        Self { buf, len: N::USIZE }
    }

    pub(crate) fn append_non_empty_string<M: NonEmptyInlineStringLength>(
        &self,
        other: &NonEmptyInlineString<M>,
    ) -> Buffer<Sum<N, M>>
    where
        N: Add<M, Output: ArrayLength>,
    {
        let mut buf = GenericArray::default();
        let len = buf
            .iter_mut()
            .zip(self.as_slice().iter().chain(other.as_bytes()))
            .map(|(dst, src)| *dst = *src)
            .count();
        Buffer { buf, len }
    }

    pub(crate) fn append_byte_array<M: ArrayLength>(
        &self,
        other: &GenericArray<u8, M>,
    ) -> Buffer<Sum<N, M>>
    where
        N: Add<M, Output: ArrayLength>,
    {
        let mut buf = GenericArray::default();
        let len = buf
            .iter_mut()
            .zip(self.as_slice().iter().chain(other))
            .map(|(dst, src)| *dst = *src)
            .count();
        Buffer { buf, len }
    }

    fn shift_left(&mut self, n: usize) {
        let mut buf = GenericArray::<u8, N>::default();
        for (dst, src) in buf.iter_mut().zip(self.buf.iter().skip(n)) {
            *dst = *src;
        }
        self.buf = buf;
        self.len = self.len.saturating_sub(n);
    }

    pub fn push(&mut self, byte: u8) -> PushResult<'_, N> {
        let Some((slot, len)) = self.buf.get_mut(self.len).zip(self.len.checked_add(1)) else {
            return PushResult::Full(&self.buf);
        };
        *slot = byte;
        self.len = len;
        PushResult::Pushed
    }

    pub fn drop_n_front_bytes(&mut self, n: usize) -> Result<(), usize> {
        if n > self.len {
            return Err(self.len);
        }
        self.shift_left(n);
        Ok(())
    }

    pub fn drain_front<E>(
        &mut self,
        mut f: impl FnMut(u8) -> Result<ControlFlow<()>, E>,
    ) -> Result<(), E> {
        let mut consumed = self.len;
        for (n, &byte) in (1..=self.len).zip(self.as_slice()) {
            if f(byte)?.is_break() {
                consumed = n;
                break;
            }
        }
        self.shift_left(consumed);
        Ok(())
    }

    pub fn clear(&mut self) {
        *self = Self::empty();
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.buf
            .get(..self.len)
            .unwrap_or_else(|| unreachable!("Buffer always has valid len"))
    }
}

impl<N: ArrayLength> Default for Buffer<N> {
    fn default() -> Self {
        Self::empty()
    }
}

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum PushResult<'a, N: ArrayLength> {
    Pushed,
    Full(&'a GenericArray<u8, N>),
}

#[cfg(test)]
mod tests {
    use super::{Buffer, PushResult};
    use alloc::vec;
    use core::ops::ControlFlow::{Break, Continue};
    use generic_array::GenericArray;
    use typenum::{U3, U4, U5};

    #[test]
    fn test_push() {
        let mut buf = Buffer::<U3>::empty();
        assert_eq!(buf.as_slice(), b"");

        assert_eq!(buf.push(b'a'), PushResult::Pushed);
        assert_eq!(buf.push(b'b'), PushResult::Pushed);
        assert_eq!(buf.push(b'c'), PushResult::Pushed);
        assert_eq!(
            buf.push(b'd'),
            PushResult::Full(&GenericArray::from_array(*b"abc"))
        );

        assert_eq!(buf.as_slice(), b"abc");
    }

    #[test]
    fn test_clear() {
        let mut buf = Buffer::<U3>::empty();
        assert_eq!(buf.push(b'a'), PushResult::Pushed);
        assert_eq!(buf.push(b'b'), PushResult::Pushed);

        buf.clear();
        assert_eq!(buf, Buffer::empty());
    }

    #[test]
    fn test_drop_n_front_bytes() {
        let mut buf = Buffer::<U4>::new(GenericArray::from_array(*b"abcd"));

        assert_eq!(buf.drop_n_front_bytes(1), Ok(()));
        assert_eq!(buf.as_slice(), b"bcd");

        assert_eq!(buf.drop_n_front_bytes(4), Err(3));
        assert_eq!(buf.as_slice(), b"bcd");

        assert_eq!(buf.drop_n_front_bytes(3), Ok(()));
        assert_eq!(buf, Buffer::empty());
    }

    #[test]
    fn test_drain_front() {
        let mut buf = Buffer::<U5>::new(GenericArray::from_array(*b"ab|cd"));
        let mut seen = vec![];
        let res: Result<(), ()> = buf.drain_front(|byte| {
            seen.push(byte);
            Ok(if byte == b'|' {
                Break(())
            } else {
                Continue(())
            })
        });
        assert_eq!(res, Ok(()));
        assert_eq!(seen, b"ab|");
        assert_eq!(buf.as_slice(), b"cd");

        let res: Result<(), ()> = buf.drain_front(|_| Ok(Continue(())));
        assert_eq!(res, Ok(()));
        assert_eq!(buf, Buffer::empty());

        let mut buf = Buffer::<U3>::new(GenericArray::from_array(*b"abc"));
        assert_eq!(buf.drain_front(|_| Err("boom")), Err("boom"));
        assert_eq!(buf.as_slice(), b"abc");
    }

    #[test]
    fn test_from_slice() {
        assert_eq!(
            Buffer::<U3>::from_slice(b"ab").map(|buf| buf.as_slice().to_vec()),
            Some(b"ab".to_vec())
        );
        assert_eq!(
            Buffer::<U3>::from_slice(b"abc").map(|buf| buf.as_slice().to_vec()),
            Some(b"abc".to_vec())
        );
        assert_eq!(Buffer::<U3>::from_slice(b"abcd"), None);
    }
}
