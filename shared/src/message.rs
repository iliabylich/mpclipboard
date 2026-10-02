use crate::NonEmptyInlineString;
use core::{num::NonZeroUsize, str::Utf8Error};

const MAX_TEXT_LEN: usize = 255;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Message {
    pub(crate) string: NonEmptyInlineString<MAX_TEXT_LEN>,
}

impl Message {
    pub const BYTESIZE: usize = {
        let size = size_of::<u8>() + MAX_TEXT_LEN;
        assert!(size == 256);
        size
    };

    pub const fn new(string: NonEmptyInlineString<MAX_TEXT_LEN>) -> Self {
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
    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let len = self.string.len().get();

        let mut buf = [0; Self::BYTESIZE];
        buf[0] = len;
        buf[1..Self::BYTESIZE].copy_from_slice(self.string.as_fixed_size_bytes());

        buf
    }

    pub(crate) fn decode(buf: &[u8; Self::BYTESIZE]) -> Result<Self, MessageError> {
        let [len, text @ ..] = buf;

        let len = NonZeroUsize::new(usize::from(*len)).ok_or(MessageError::Empty)?;
        let text = text
            .get(..len.get())
            .unwrap_or_else(|| unreachable!("len is a u8, so it never exceeds MAX_TEXT_LEN"));
        let text = core::str::from_utf8(text).map_err(MessageError::NonUtf8)?;
        let string = NonEmptyInlineString::new(text)
            .unwrap_or_else(|_| unreachable!("text is non-empty and never exceeds MAX_TEXT_LEN"));

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
    NonUtf8(Utf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    type S = NonEmptyInlineString<MAX_TEXT_LEN>;

    #[test]
    fn test_encode_decode() {
        let text = Message::new(S::const_new("aaaaaaaaaa"));
        assert_eq!(Message::decode(&text.encode()), Ok(text));
    }

    #[test]
    fn test_decode_invalid() {
        assert_eq!(
            Message::decode(&[0; Message::BYTESIZE]),
            Err(MessageError::Empty)
        );
        assert!(matches!(
            Message::decode(&[b'\xC8'; Message::BYTESIZE]),
            Err(MessageError::NonUtf8(_))
        ));
    }
}
