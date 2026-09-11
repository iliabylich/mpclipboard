use core::num::NonZeroU8;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonEmptyInlineString<const MAXLEN: usize> {
    len: NonZeroU8,
    bytes: [u8; MAXLEN],
}

impl<const MAXLEN: usize> NonEmptyInlineString<MAXLEN> {
    pub fn truncate(s: &str) -> Result<Self, NonEmptyInlineStringTruncateError> {
        let mut bytes = [0; MAXLEN];
        let minlen = core::cmp::min(s.len(), MAXLEN);

        let src = s
            .as_bytes()
            .get(..minlen)
            .unwrap_or_else(|| unreachable!("minlen is capped by strings's length"));
        let src = match core::str::from_utf8(src) {
            Ok(s) => s.as_bytes(),
            Err(err) => s
                .as_bytes()
                .get(..err.valid_up_to())
                .unwrap_or_else(|| unreachable!("str must be valid up to len")),
        };

        let len = u8::try_from(src.len())
            .map_err(|_| NonEmptyInlineStringTruncateError::LenParamIsTooLong)?;
        let len = NonZeroU8::new(len).ok_or(NonEmptyInlineStringTruncateError::StringIsEmpty)?;
        let dst = bytes
            .get_mut(..usize::from(len.get()))
            .unwrap_or_else(|| unreachable!("len <= MAXLEN"));
        dst.copy_from_slice(src);
        Ok(Self { len, bytes })
    }

    pub fn new(s: &str) -> Result<Self, NonEmptyInlineStringNewError> {
        let len =
            u8::try_from(s.len()).map_err(|_| NonEmptyInlineStringNewError::StringIsTooLong)?;
        let len = NonZeroU8::new(len).ok_or(NonEmptyInlineStringNewError::StringIsEmpty)?;

        let mut bytes = [0; MAXLEN];
        let src = bytes
            .get_mut(..usize::from(len.get()))
            .ok_or(NonEmptyInlineStringNewError::StringIsTooLong)?;
        src.copy_from_slice(s.as_bytes());
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
}

impl<const MAXLEN: usize> core::fmt::Debug for NonEmptyInlineString<MAXLEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const MAXLEN: usize> core::fmt::Display for NonEmptyInlineString<MAXLEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NonEmptyInlineStringTruncateError {
    LenParamIsTooLong,
    StringIsEmpty,
}

impl core::fmt::Display for NonEmptyInlineStringTruncateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LenParamIsTooLong => f.write_str("LenParamIsTooLong"),
            Self::StringIsEmpty => f.write_str("StringIsEmpty"),
        }
    }
}

impl core::error::Error for NonEmptyInlineStringTruncateError {}

#[derive(Debug, PartialEq, Eq)]
pub enum NonEmptyInlineStringNewError {
    LenParamIsTooLong,
    StringIsEmpty,
    StringIsTooLong,
}

impl core::fmt::Display for NonEmptyInlineStringNewError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LenParamIsTooLong => f.write_str("LenParamIsTooLong"),
            Self::StringIsEmpty => f.write_str("StringIsEmpty"),
            Self::StringIsTooLong => f.write_str("StringIsTooLong"),
        }
    }
}

impl core::error::Error for NonEmptyInlineStringNewError {}

#[cfg(test)]
mod tess {
    use super::*;

    #[test]
    fn test_short() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcde")
                .unwrap()
                .as_str(),
            "abcde"
        );
    }

    #[test]
    fn test_long() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcdef")
                .unwrap()
                .as_str(),
            "abcde"
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("ႠႠႠႠ")
                .unwrap()
                .as_str(),
            "ႠႠႠ"
        );

        assert_eq!('🦴'.len_utf8(), 4);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("🦴🦴🦴")
                .unwrap()
                .as_str(),
            "🦴🦴"
        );
    }

    #[test]
    fn test_err() {
        assert_eq!(
            NonEmptyInlineString::<100>::truncate("").unwrap_err(),
            NonEmptyInlineStringTruncateError::StringIsEmpty
        );
    }
}
