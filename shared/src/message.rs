use crate::NonEmptyInlineString;
use anyhow::{Context, Result};
use core::num::NonZeroUsize;

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

    pub const fn new(string: NonEmptyInlineString<MAX_TEXT_LEN>) -> Result<Self> {
        Ok(Self { string })
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

    pub(crate) fn decode(buf: &[u8; Self::BYTESIZE]) -> Result<Self> {
        let len = buf[0];

        let mut bytes: [u8; MAX_TEXT_LEN] = [0; _];
        bytes.copy_from_slice(&buf[1..Self::BYTESIZE]);

        let len = NonZeroUsize::new(usize::from(len)).context("malformed message length")?;
        let bytes = bytes.get(..len.get()).context("malformed message length")?;
        let text = core::str::from_utf8(bytes).context("non-utf8 message text")?;
        let string = NonEmptyInlineString::new(text).context("bug")?;

        Ok(Self { string })
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text({:?})", self.text_as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type S = NonEmptyInlineString<MAX_TEXT_LEN>;

    #[test]
    fn test_encode_decode() -> Result<()> {
        let text = Message::new(S::new(&"a".repeat(10))?)?;

        assert_eq!(Message::decode(&text.encode())?, text);
        Ok(())
    }

    #[test]
    fn test_decode_invalid() {
        assert_eq!(
            Message::decode(&[0; Message::BYTESIZE]).map_err(|err| err.to_string()),
            Err("malformed message length".to_string())
        );

        assert_eq!(
            Message::decode(&[b'\xC8'; Message::BYTESIZE]).map_err(|err| err.to_string()),
            Err("non-utf8 message text".to_string())
        );
    }
}
