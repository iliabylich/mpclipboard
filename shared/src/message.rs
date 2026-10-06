use crate::NonEmptyInlineString;
use core::{num::NonZeroUsize, str::Utf8Error};
use generic_array::{
    GenericArray,
    sequence::{Lengthen, Shorten},
};
use typenum::{U1, U255, Unsigned, op};

type MaxTextLength = U255;
pub(crate) type MessageSize = op!(MaxTextLength + U1);
const _: () = assert!(MessageSize::USIZE == 256);

#[must_use]
#[derive(Clone, PartialEq, Eq)]
pub struct Message {
    pub(crate) string: NonEmptyInlineString<MaxTextLength>,
}

impl Message {
    pub const fn new(string: NonEmptyInlineString<MaxTextLength>) -> Self {
        Self { string }
    }

    #[must_use]
    pub fn text_as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    #[must_use]
    pub fn text_as_str(&self) -> &str {
        self.string.as_str()
    }

    #[must_use]
    pub(crate) fn encode(&self) -> GenericArray<u8, MessageSize> {
        let len = self.string.len().get();
        self.string.as_fixed_size_bytes().prepend(len)
    }

    pub(crate) fn decode(buf: GenericArray<u8, MessageSize>) -> Result<Self, MessageError> {
        let (len, text) = buf.pop_front();

        let len = NonZeroUsize::new(usize::from(len)).ok_or(MessageError::Empty)?;
        let text = text
            .get(..len.get())
            .unwrap_or_else(|| unreachable!("len is a u8, so it never exceeds MaxTextLength"));
        let text = core::str::from_utf8(text)?;
        let string = NonEmptyInlineString::new(text)
            .unwrap_or_else(|_| unreachable!("text is non-empty and never exceeds MaxTextLength"));

        Ok(Self { string })
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text({:?})", self.text_as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("malformed message length")]
    Empty,
    #[error("non-utf8 message text: {0}")]
    NonUtf8(#[from] Utf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches;

    type S = NonEmptyInlineString<MaxTextLength>;

    #[test]
    fn test_encode_decode() {
        let text = Message::new(S::const_new("aaaaaaaaaa"));
        assert_eq!(Message::decode(text.encode()), Ok(text));
    }

    #[test]
    fn test_decode_invalid() {
        assert_eq!(
            Message::decode(GenericArray::from_array([0; MessageSize::USIZE])),
            Err(MessageError::Empty)
        );
        assert_matches!(
            Message::decode(GenericArray::from_array([b'\xC8'; MessageSize::USIZE])),
            Err(MessageError::NonUtf8(_))
        );
    }
}
