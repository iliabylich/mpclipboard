use crate::NonEmptyInlineString;
use core::num::NonZeroUsize;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TEXT_LEN: usize = 200;

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Message {
    pub(crate) string: NonEmptyInlineString<MAX_TEXT_LEN>,
    pub(crate) timestamp: u128,
}

impl Message {
    pub const BYTESIZE: usize = size_of::<u8>() + size_of::<u128>() + MAX_TEXT_LEN;

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
        let mut buf = [0; Self::BYTESIZE];
        let text = self.text_as_bytes();
        let len = u8::try_from(text.len()).unwrap_or_else(|_| unreachable!());

        let mut pos = 0_usize;

        let start = pos;
        let end = start
            .checked_add(size_of::<u8>())
            .unwrap_or_else(|| unreachable!());
        buf.get_mut(start..end)
            .unwrap_or_else(|| unreachable!())
            .copy_from_slice(&len.to_le_bytes());
        pos = end;

        let start = pos;
        let end = start
            .checked_add(size_of::<u128>())
            .unwrap_or_else(|| unreachable!());
        buf.get_mut(start..end)
            .unwrap_or_else(|| unreachable!())
            .copy_from_slice(&self.timestamp.to_le_bytes());
        pos = end;

        let start = pos;
        let end = start
            .checked_add(len as usize)
            .unwrap_or_else(|| unreachable!());
        buf.get_mut(start..end)
            .unwrap_or_else(|| unreachable!())
            .copy_from_slice(text);

        buf
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text({:?} at {})", self.text_as_str(), self.timestamp)
    }
}

impl Message {
    pub(crate) fn decode(buf: &[u8; Self::BYTESIZE]) -> Result<Self, MessageDecodeError> {
        let (length, buf) = buf.split_first().unwrap_or_else(|| unreachable!());
        let length =
            NonZeroUsize::new(usize::from(*length)).ok_or(MessageDecodeError::MalformedLength)?;

        if length.get() > MAX_TEXT_LEN {
            return Err(MessageDecodeError::MalformedLength);
        }

        let (timestamp, buf) = buf
            .split_first_chunk::<{ size_of::<u128>() }>()
            .unwrap_or_else(|| unreachable!());
        let timestamp = u128::from_le_bytes(*timestamp);

        let buf = buf.get(..length.get()).unwrap_or_else(|| unreachable!());
        let mut text = [0; MAX_TEXT_LEN];
        text.get_mut(..length.get())
            .unwrap_or_else(|| unreachable!())
            .copy_from_slice(buf);

        let string =
            core::str::from_utf8(text.get(..length.get()).unwrap_or_else(|| unreachable!()))
                .map_err(|_| MessageDecodeError::NonUtf8Text)?;
        let string = NonEmptyInlineString::new(string).unwrap_or_else(|_| unreachable!());

        Ok(Self { string, timestamp })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDecodeError {
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
            Message::decode(&[b'\xFF'; Message::BYTESIZE])
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
