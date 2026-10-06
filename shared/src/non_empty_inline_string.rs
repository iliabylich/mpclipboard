use const_default::ConstDefault;
use core::num::NonZeroU8;
use generic_array::{ArrayLength, GenericArray};
use typenum::{IsLessOrEqual, NonZero, True, U255};

pub trait NonEmptyInlineStringLength:
    ArrayLength + NonZero + IsLessOrEqual<U255, Output = True>
{
}

impl<N: ArrayLength + NonZero + IsLessOrEqual<U255, Output = True>> NonEmptyInlineStringLength
    for N
{
}

#[must_use]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyInlineString<N: NonEmptyInlineStringLength> {
    len: NonZeroU8,
    bytes: GenericArray<u8, N>,
}

impl<N: NonEmptyInlineStringLength> NonEmptyInlineString<N> {
    pub fn truncate(s: &str) -> Result<Self, NonEmptyInlineStringError> {
        let (head, _tail) = s.split_at(s.floor_char_boundary(N::USIZE));
        Self::new(head)
    }

    pub fn new(s: &str) -> Result<Self, NonEmptyInlineStringError> {
        let len = u8::try_from(s.len()).map_err(|_| NonEmptyInlineStringError::TooLong)?;
        let len = NonZeroU8::new(len).ok_or(NonEmptyInlineStringError::Empty)?;

        let mut bytes = GenericArray::default();
        bytes
            .get_mut(..s.len())
            .ok_or(NonEmptyInlineStringError::TooLong)?
            .copy_from_slice(s.as_bytes());

        Ok(Self { len, bytes })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes
            .get(..usize::from(self.len.get()))
            .unwrap_or_else(|| unreachable!("NonEmptyInlineString always has valid len"))
    }

    #[must_use]
    pub const fn as_fixed_size_bytes(&self) -> &GenericArray<u8, N> {
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
    pub const fn const_new(s: &str) -> Self
    where
        GenericArray<u8, N>: ConstDefault,
    {
        assert!(!s.is_empty(), "empty string");
        assert!(s.len() <= N::USIZE, "string is too long");

        let mut bytes = GenericArray::DEFAULT;
        let (head, _tail) = bytes.as_mut_slice().split_at_mut(s.len());
        head.copy_from_slice(s.as_bytes());

        #[expect(clippy::cast_possible_truncation)]
        let Some(len) = NonZeroU8::new(s.len() as u8) else {
            panic!("empty string, checked above");
        };
        Self { len, bytes }
    }
}

impl<N: NonEmptyInlineStringLength> core::fmt::Debug for NonEmptyInlineString<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl<N: NonEmptyInlineStringLength> core::fmt::Display for NonEmptyInlineString<N> {
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
    use typenum::{U3, U5, U10, U100};

    #[test]
    fn test_short() {
        assert_eq!(
            NonEmptyInlineString::<U5>::truncate("abcde")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("abcde")
        );
    }

    #[test]
    fn test_long() {
        assert_eq!(
            NonEmptyInlineString::<U5>::truncate("abcdef")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("abcde")
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        assert_eq!(
            NonEmptyInlineString::<U10>::truncate("ႠႠႠႠ")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("ႠႠႠ")
        );

        assert_eq!('🦴'.len_utf8(), 4);
        assert_eq!(
            NonEmptyInlineString::<U10>::truncate("🦴🦴🦴")
                .as_ref()
                .map(NonEmptyInlineString::as_str),
            Ok("🦴🦴")
        );
    }

    #[test]
    fn test_err() {
        assert_eq!(
            NonEmptyInlineString::<U100>::truncate(""),
            Err(NonEmptyInlineStringError::Empty)
        );
        assert_eq!(
            NonEmptyInlineString::<U3>::new(""),
            Err(NonEmptyInlineStringError::Empty)
        );
        assert_eq!(
            NonEmptyInlineString::<U3>::new("abcd"),
            Err(NonEmptyInlineStringError::TooLong)
        );
        assert_eq!(
            NonEmptyInlineString::<U3>::new(&"a".repeat(300)),
            Err(NonEmptyInlineStringError::TooLong)
        );
    }

    #[test]
    fn test_const_new() {
        type Ten = NonEmptyInlineString<U10>;

        const S1: Ten = Ten::const_new("foobarbaz0");
        const S2: Ten = Ten::const_new("abc");

        assert_eq!(S1.as_str(), "foobarbaz0");
        assert_eq!(S2.as_str(), "abc");
    }
}
