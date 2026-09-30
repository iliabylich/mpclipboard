#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Buffer<const MAXLEN: usize> {
    buf: [u8; MAXLEN],
    len: usize,
}

impl<const MAXLEN: usize> Buffer<MAXLEN> {
    pub const fn new() -> Self {
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

    #[must_use]
    pub fn push(&mut self, byte: u8) -> bool {
        let Some((slot, len)) = self.buf.get_mut(self.len).zip(self.len.checked_add(1)) else {
            return false;
        };
        *slot = byte;
        self.len = len;
        true
    }

    pub const fn clear(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        let Some(bytes) = self.buf.get(..self.len) else {
            unreachable!("Buffer always has valid len");
        };
        bytes
    }

    #[must_use]
    pub const fn as_full_array(&self) -> Option<&[u8; MAXLEN]> {
        if self.len == MAXLEN {
            Some(&self.buf)
        } else {
            None
        }
    }
}

impl<const MAXLEN: usize> Default for Buffer<MAXLEN> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;

    #[test]
    fn test_push() {
        let mut buf = Buffer::<3>::new();
        assert_eq!(buf.as_slice(), b"");

        assert!(buf.push(b'a'));
        assert!(buf.push(b'b'));
        assert!(buf.push(b'c'));
        assert!(!buf.push(b'd'));

        assert_eq!(buf.as_slice(), b"abc");
    }

    #[test]
    fn test_clear() {
        let mut buf = Buffer::<3>::new();
        assert!(buf.push(b'a'));
        assert!(buf.push(b'b'));

        buf.clear();
        assert_eq!(buf, Buffer::new());
    }

    #[test]
    fn test_as_full_array() {
        let mut buf = Buffer::<2>::new();
        assert_eq!(buf.as_full_array(), None);

        assert!(buf.push(b'a'));
        assert_eq!(buf.as_full_array(), None);

        assert!(buf.push(b'b'));
        assert_eq!(buf.as_full_array(), Some(b"ab"));
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
