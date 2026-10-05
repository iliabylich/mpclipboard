use core::ops::ControlFlow;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Buffer<const MAXLEN: usize> {
    buf: [u8; MAXLEN],
    len: usize,
}

impl<const MAXLEN: usize> Buffer<MAXLEN> {
    pub const fn empty() -> Self {
        Self {
            buf: [0; MAXLEN],
            len: 0,
        }
    }

    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; MAXLEN];
        buf.get_mut(..bytes.len())?.copy_from_slice(bytes);
        Some(Self {
            buf,
            len: bytes.len(),
        })
    }

    pub const fn new(buf: [u8; MAXLEN]) -> Self {
        Self { buf, len: MAXLEN }
    }

    fn from_suffix(suffix: &[u8]) -> Self {
        assert!(suffix.len() <= MAXLEN, "suffix doesn't fit into Buffer");

        let mut buf = [0; MAXLEN];
        for (dst, src) in buf.iter_mut().zip(suffix) {
            *dst = *src;
        }
        Self {
            buf,
            len: suffix.len(),
        }
    }

    pub fn push(&mut self, byte: u8) -> PushResult<'_, MAXLEN> {
        let Some((slot, len)) = self.buf.get_mut(self.len).zip(self.len.checked_add(1)) else {
            return PushResult::Full(&self.buf);
        };
        *slot = byte;
        self.len = len;
        PushResult::Pushed
    }

    pub fn drop_n_front_bytes(&mut self, n: usize) -> Result<(), usize> {
        let Some(rest) = self.as_slice().get(n..) else {
            return Err(self.len);
        };
        *self = Self::from_suffix(rest);
        Ok(())
    }

    pub fn drain_front<E>(
        &mut self,
        mut f: impl FnMut(u8) -> Result<ControlFlow<()>, E>,
    ) -> Result<(), E> {
        let mut bytes = self.as_slice().iter();
        for &byte in bytes.by_ref() {
            if f(byte)?.is_break() {
                break;
            }
        }
        *self = Self::from_suffix(bytes.as_slice());
        Ok(())
    }

    pub const fn clear(&mut self) {
        *self = Self::empty();
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.buf
            .get(..self.len)
            .unwrap_or_else(|| unreachable!("Buffer always has valid len"))
    }
}

impl<const MAXLEN: usize> Default for Buffer<MAXLEN> {
    fn default() -> Self {
        Self::empty()
    }
}

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum PushResult<'a, const MAXLEN: usize> {
    Pushed,
    Full(&'a [u8; MAXLEN]),
}

#[cfg(test)]
mod tests {
    use super::{Buffer, PushResult};
    use alloc::vec;
    use core::ops::ControlFlow::{Break, Continue};

    #[test]
    fn test_push() {
        let mut buf = Buffer::<3>::empty();
        assert_eq!(buf.as_slice(), b"");

        assert_eq!(buf.push(b'a'), PushResult::Pushed);
        assert_eq!(buf.push(b'b'), PushResult::Pushed);
        assert_eq!(buf.push(b'c'), PushResult::Pushed);
        assert_eq!(buf.push(b'd'), PushResult::Full(b"abc"));

        assert_eq!(buf.as_slice(), b"abc");
    }

    #[test]
    fn test_clear() {
        let mut buf = Buffer::<3>::empty();
        assert_eq!(buf.push(b'a'), PushResult::Pushed);
        assert_eq!(buf.push(b'b'), PushResult::Pushed);

        buf.clear();
        assert_eq!(buf, Buffer::empty());
    }

    #[test]
    fn test_drop_n_front_bytes() {
        let mut buf = Buffer::<4>::new(*b"abcd");

        assert_eq!(buf.drop_n_front_bytes(1), Ok(()));
        assert_eq!(buf.as_slice(), b"bcd");

        assert_eq!(buf.drop_n_front_bytes(4), Err(3));
        assert_eq!(buf.as_slice(), b"bcd");

        assert_eq!(buf.drop_n_front_bytes(3), Ok(()));
        assert_eq!(buf, Buffer::empty());
    }

    #[test]
    fn test_drain_front() {
        let mut buf = Buffer::<5>::new(*b"ab|cd");
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

        let mut buf = Buffer::<3>::new(*b"abc");
        assert_eq!(buf.drain_front(|_| Err("boom")), Err("boom"));
        assert_eq!(buf.as_slice(), b"abc");
    }

    #[test]
    fn test_from_slice() {
        assert_eq!(
            Buffer::<3>::from_slice(b"ab").map(|buf| buf.as_slice().to_vec()),
            Some(b"ab".to_vec())
        );
        assert_eq!(
            Buffer::<3>::from_slice(b"abc").map(|buf| buf.as_slice().to_vec()),
            Some(b"abc".to_vec())
        );
        assert_eq!(Buffer::<3>::from_slice(b"abcd"), None);
    }
}
