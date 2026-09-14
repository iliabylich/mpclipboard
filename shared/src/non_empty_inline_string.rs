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
}
