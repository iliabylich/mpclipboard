use crate::NonEmptyInlineString;
use core::num::NonZeroUsize;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TEXT_LEN: usize = 255;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Message {
    pub(crate) string: NonEmptyInlineString<MAX_TEXT_LEN>,
    pub(crate) timestamp: u128,
}

impl Message {
    pub const BYTESIZE: usize = {
        let size = size_of::<u8>() + size_of::<u128>() + MAX_TEXT_LEN;
        assert!(size == 272);
        size
    };

    pub fn new(string: NonEmptyInlineString<MAX_TEXT_LEN>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| unreachable!("bug: time goes backwards"))
            .as_nanos();

        Self { string, timestamp }
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
    pub const fn timestamp(&self) -> u128 {
        self.timestamp
    }

    #[must_use]
    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let len = self.string.len().get();
        let timestamp: [u8; 16] = self.timestamp.to_le_bytes();

        let mut buf = [0; Self::BYTESIZE];
        buf[0] = len;
        buf[1..17].copy_from_slice(&timestamp);
        buf[17..Self::BYTESIZE].copy_from_slice(self.string.as_fixed_size_bytes());

        buf
    }

    pub(crate) fn decode(buf: &[u8; Self::BYTESIZE]) -> Result<Self, MessageDecodeError> {
        let len = buf[0];

        let mut timestamp: [u8; 16] = [0; _];
        timestamp.copy_from_slice(&buf[1..17]);
        let timestamp = u128::from_le_bytes(timestamp);

        let mut bytes: [u8; MAX_TEXT_LEN] = [0; _];
        bytes.copy_from_slice(&buf[17..Self::BYTESIZE]);

        let len = NonZeroUsize::new(usize::from(len)).ok_or(MessageDecodeError::MalformedLength)?;
        let bytes = bytes
            .get(..len.get())
            .ok_or(MessageDecodeError::MalformedLength)?;
        let text = core::str::from_utf8(bytes).map_err(|_| MessageDecodeError::NonUtf8Text)?;
        let string = NonEmptyInlineString::new(text).unwrap_or_else(|_| unreachable!());

        Ok(Self { string, timestamp })
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text({:?} at {})", self.text_as_str(), self.timestamp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageDecodeError {
    MalformedLength,
    NonUtf8Text,
}

impl core::fmt::Display for MessageDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MalformedLength => f.write_str("malformed message length"),
            Self::NonUtf8Text => f.write_str("non-utf8 message text"),
        }
    }
}

impl core::error::Error for MessageDecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    type S = NonEmptyInlineString<MAX_TEXT_LEN>;

    #[test]
    fn test_encode_decode() {
        let text = Message::new(S::new(&"a".repeat(10)).unwrap());

        assert_eq!(Message::decode(&text.encode()).unwrap(), text);
    }

    #[test]
    fn test_decode_invalid() {
        assert_eq!(
            Message::decode(&[0; Message::BYTESIZE])
                .unwrap_err()
                .to_string(),
            "malformed message length"
        );

        assert_eq!(
            Message::decode(&[b'\xC8'; Message::BYTESIZE])
                .unwrap_err()
                .to_string(),
            "non-utf8 message text"
        );
    }
}
