use core::num::NonZeroUsize;

#[derive(Clone, Copy)]
pub(crate) struct FixedStr<const N: usize> {
    buf: [u8; N],
    len: NonZeroUsize,
}

impl<const N: usize> FixedStr<N> {
    pub(crate) fn new(s: &str) -> Result<Self, FixedStrError> {
        let len = NonZeroUsize::new(s.len()).ok_or(FixedStrError::Empty)?;
        if len.get() > N {
            return Err(FixedStrError::TooLong { len: len.get() });
        }

        let mut buf = [0; N];
        buf[..len.get()].copy_from_slice(s.as_bytes());
        Ok(Self { buf, len })
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len.get()]
    }

    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes())
            .unwrap_or_else(|_| unreachable!("FixedStr is always built from a valid &str"))
    }
}

impl<const N: usize> PartialEq for FixedStr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<const N: usize> Eq for FixedStr<N> {}

impl<const N: usize> core::fmt::Debug for FixedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl<const N: usize> core::fmt::Display for FixedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedStrError {
    Empty,
    TooLong { len: usize },
}

impl core::fmt::Display for FixedStrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("string is empty"),
            Self::TooLong { len } => write!(f, "string is {len} bytes long, which is too long"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FixedStr, FixedStrError};

    #[test]
    fn test_new() {
        assert_eq!(
            FixedStr::<3>::new("abc").map(|s| s.as_str() == "abc"),
            Ok(true)
        );
        assert_eq!(
            FixedStr::<8>::new("abc").map(|s| s.as_str() == "abc"),
            Ok(true)
        );
    }

    #[test]
    fn test_multibyte() {
        assert_eq!(FixedStr::<3>::new("Ⴀ").map(|s| s.as_str() == "Ⴀ"), Ok(true));
        assert_eq!(
            FixedStr::<2>::new("Ⴀ"),
            Err(FixedStrError::TooLong { len: 3 })
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(FixedStr::<3>::new(""), Err(FixedStrError::Empty));
    }

    #[test]
    fn test_too_long() {
        assert_eq!(
            FixedStr::<3>::new("abcd"),
            Err(FixedStrError::TooLong { len: 4 })
        );
    }

    #[test]
    fn test_eq_ignores_capacity_tail() {
        assert_eq!(FixedStr::<8>::new("abc"), FixedStr::<8>::new("abc"));
        assert_ne!(FixedStr::<8>::new("abc"), FixedStr::<8>::new("abcd"));
    }
}
