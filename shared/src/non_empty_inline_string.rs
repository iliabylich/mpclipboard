use anyhow::{Context, Result};
use core::num::NonZeroU8;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonEmptyInlineString<const MAXLEN: usize> {
    len: NonZeroU8,
    bytes: [u8; MAXLEN],
}

impl<const MAXLEN: usize> NonEmptyInlineString<MAXLEN> {
    pub fn truncate(s: &str) -> Result<Self> {
        let mut bytes = [0; MAXLEN];
        let minlen = core::cmp::min(s.len(), MAXLEN);

        let src = s
            .as_bytes()
            .get(..minlen)
            .context("minlen is capped by strings's length")?;
        let src = match core::str::from_utf8(src) {
            Ok(s) => s.as_bytes(),
            Err(err) => s
                .as_bytes()
                .get(..err.valid_up_to())
                .context("str must be valid up to len")?,
        };

        let len = u8::try_from(src.len()).context("MAXLEN param is too long")?;
        let len = NonZeroU8::new(len).context("string is empty")?;
        let dst = bytes
            .get_mut(..usize::from(len.get()))
            .context("len <= MAXLEN")?;
        dst.copy_from_slice(src);
        Ok(Self { len, bytes })
    }

    pub fn new(s: &str) -> Result<Self> {
        let len = u8::try_from(s.len()).context("string is too long")?;
        let len = NonZeroU8::new(len).context("string is empty")?;

        let mut bytes = [0; MAXLEN];
        let src = bytes
            .get_mut(..usize::from(len.get()))
            .context("string is too long")?;
        src.copy_from_slice(s.as_bytes());
        Ok(Self { len, bytes })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        let Some(bytes) = self.bytes.get(..usize::from(self.len.get())) else {
            unreachable!("NonEmptyInlineString always has valid len");
        };
        bytes
    }

    #[must_use]
    pub const fn as_fixed_size_bytes(&self) -> &[u8; MAXLEN] {
        &self.bytes
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        let Ok(s) = core::str::from_utf8(self.as_bytes()) else {
            unreachable!("NonEmptyInlineString is always a UTF-8 valid string");
        };
        s
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
        assert!(!s.is_empty(), "empty string");
        assert!(s.len() < u8::MAX as usize, "string is too long");
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

#[cfg(test)]
mod tess {
    use super::*;

    #[test]
    fn test_short() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcde")
                .expect("must be valid")
                .as_str(),
            "abcde"
        );
    }

    #[test]
    fn test_long() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcdef")
                .expect("must be valid")
                .as_str(),
            "abcde"
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("ႠႠႠႠ")
                .expect("must be valid")
                .as_str(),
            "ႠႠႠ"
        );

        assert_eq!('🦴'.len_utf8(), 4);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("🦴🦴🦴")
                .expect("must be valid")
                .as_str(),
            "🦴🦴"
        );
    }

    #[test]
    fn test_err() {
        assert_eq!(
            NonEmptyInlineString::<100>::truncate("")
                .expect_err("empty")
                .to_string(),
            "string is empty"
        );
    }

    #[test]
    fn test_const_new() {
        type Ten = NonEmptyInlineString<10>;

        const S1: Ten = Ten::const_new("foobarbaz0");
        assert_eq!(S1.as_str(), "foobarbaz0");

        const S2: Ten = Ten::const_new("abc");
        assert_eq!(S2.as_str(), "abc");
    }
}
