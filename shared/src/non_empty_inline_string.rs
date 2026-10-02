use core::num::NonZeroU8;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonEmptyInlineString<const MAXLEN: usize> {
    len: NonZeroU8,
    bytes: [u8; MAXLEN],
}

impl<const MAXLEN: usize> NonEmptyInlineString<MAXLEN> {
    const MAXLEN_FITS_INTO_U8: () = assert!(MAXLEN <= u8::MAX as usize, "MAXLEN must fit into u8");

    pub fn truncate(s: &str) -> Result<Self, NonEmptyInlineStringError> {
        let (head, _tail) = s.split_at(s.floor_char_boundary(MAXLEN));
        Self::new(head)
    }

    pub fn new(s: &str) -> Result<Self, NonEmptyInlineStringError> {
        let () = Self::MAXLEN_FITS_INTO_U8;

        let mut bytes = [0; MAXLEN];
        bytes
            .get_mut(..s.len())
            .ok_or(NonEmptyInlineStringError::TooLong)?
            .copy_from_slice(s.as_bytes());

        let len =
            u8::try_from(s.len()).unwrap_or_else(|_| unreachable!("s.len() <= MAXLEN <= u8::MAX"));
        let len = NonZeroU8::new(len).ok_or(NonEmptyInlineStringError::Empty)?;

        Ok(Self { len, bytes })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes
            .get(..usize::from(self.len.get()))
            .unwrap_or_else(|| unreachable!("NonEmptyInlineString always has valid len"))
    }

    #[must_use]
    pub const fn as_fixed_size_bytes(&self) -> &[u8; MAXLEN] {
        &self.bytes
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes())
            .unwrap_or_else(|_| unreachable!("NonEmptyInlineString is always a UTF-8 valid string"))
    }

    #[must_use]
    pub const fn len(&self) -> NonZeroU8 {
        self.len
    }

    /// # Panics
    ///
    /// Panics if given string is either empty or too long.
    ///
    /// This function is designed to be used in a const context, that's why it panics instead of returning an error.
    pub const fn const_new(s: &str) -> Self {
        let () = Self::MAXLEN_FITS_INTO_U8;
        assert!(!s.is_empty(), "empty string");
        assert!(s.len() <= MAXLEN, "string is too long");

        let mut bytes = [0; MAXLEN];
        let (head, _tail) = bytes.split_at_mut(s.len());
        head.copy_from_slice(s.as_bytes());

        #[expect(clippy::cast_possible_truncation)]
        let Some(len) = NonZeroU8::new(s.len() as u8) else {
            panic!("empty string, checked above");
        };
        Self { len, bytes }
    }
}

impl<const MAXLEN: usize> core::fmt::Debug for NonEmptyInlineString<MAXLEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl<const MAXLEN: usize> core::fmt::Display for NonEmptyInlineString<MAXLEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NonEmptyInlineStringError {
    #[error("string is empty")]
    Empty,
    #[error("string is too long")]
    TooLong,
}

#[cfg(test)]
mod tess {
    use super::*;

    #[test]
    fn test_short() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcde")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("abcde")
        );
    }

    #[test]
    fn test_long() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcdef")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("abcde")
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("ႠႠႠႠ")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("ႠႠႠ")
        );

        assert_eq!('🦴'.len_utf8(), 4);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("🦴🦴🦴")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("🦴🦴")
        );
    }

    #[test]
    fn test_err() {
        assert_eq!(
            NonEmptyInlineString::<100>::truncate(""),
            Err(NonEmptyInlineStringError::Empty)
        );
        assert_eq!(
            NonEmptyInlineString::<3>::new(""),
            Err(NonEmptyInlineStringError::Empty)
        );
        assert_eq!(
            NonEmptyInlineString::<3>::new("abcd"),
            Err(NonEmptyInlineStringError::TooLong)
        );
    }

    #[test]
    fn test_const_new() {
        type Ten = NonEmptyInlineString<10>;

        const S1: Ten = Ten::const_new("foobarbaz0");
        const S2: Ten = Ten::const_new("abc");

        assert_eq!(S1.as_str(), "foobarbaz0");
        assert_eq!(S2.as_str(), "abc");
    }
}
