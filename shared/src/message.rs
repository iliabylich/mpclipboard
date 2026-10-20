use crate::{NonEmptyString, NonEmptyStringError};
use core::{num::NonZeroUsize, str::Utf8Error};

const MAX_TEXT_LEN: usize = 255;

#[must_use]
#[derive(Clone, PartialEq, Eq)]
pub struct Message {
    text: NonEmptyString,
}

impl Message {
    pub const BYTESIZE: usize = {
        let size = size_of::<u8>() + MAX_TEXT_LEN;
        assert!(size == 256);
        size
    };

    pub fn truncate(text: &str) -> Result<Self, NonEmptyStringError> {
        let (head, _tail) = text.split_at(text.floor_char_boundary(MAX_TEXT_LEN));
        let text = NonEmptyString::new(head)?;
        Ok(Self { text })
    }

    #[must_use]
    pub const fn text_as_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    #[must_use]
    pub const fn text_as_str(&self) -> &str {
        self.text.as_str()
    }

    #[must_use]
    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let text = self.text_as_bytes();
        let len = u8::try_from(text.len())
            .unwrap_or_else(|_| unreachable!("text never exceeds MAX_TEXT_LEN"));

        let mut buf = [0; Self::BYTESIZE];
        let [len_slot, rest @ ..] = &mut buf;
        *len_slot = len;
        rest.get_mut(..text.len())
            .unwrap_or_else(|| unreachable!("text never exceeds MAX_TEXT_LEN"))
            .copy_from_slice(text);

        buf
    }
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Frame {
    Message(Message),
    Heartbeat,
}

impl Frame {
    pub(crate) fn encode(&self) -> [u8; Message::BYTESIZE] {
        match self {
            Self::Message(message) => message.encode(),
            Self::Heartbeat => [0; Message::BYTESIZE],
        }
    }

    pub(crate) fn decode(buf: &[u8; Message::BYTESIZE]) -> Result<Self, MessageError> {
        let [len, text @ ..] = buf;

        let Some(len) = NonZeroUsize::new(usize::from(*len)) else {
            return Ok(Self::Heartbeat);
        };
        let text = text
            .get(..len.get())
            .unwrap_or_else(|| unreachable!("len is a u8, so it never exceeds MAX_TEXT_LEN"));
        let text = core::str::from_utf8(text)?;
        let text = NonEmptyString::new(text).unwrap_or_else(|_| unreachable!("len is non-zero"));

        Ok(Self::Message(Message { text }))
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text({:?})", self.text_as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("non-utf8 message text: {0}")]
    NonUtf8(#[from] Utf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches;

    #[test]
    fn test_encode_decode() {
        let text = Frame::Message(Message::truncate("aaaaaaaaaa").unwrap());
        assert_eq!(Frame::decode(&text.encode()), Ok(text));
    }

    #[test]
    fn test_encode_decode_max_len() {
        let text = Frame::Message(Message::truncate(&"a".repeat(MAX_TEXT_LEN)).unwrap());
        assert_eq!(Frame::decode(&text.encode()), Ok(text));
    }

    #[test]
    fn test_heartbeat() {
        assert_eq!(Frame::Heartbeat.encode(), [0; Message::BYTESIZE]);
        assert_eq!(
            Frame::decode(&Frame::Heartbeat.encode()),
            Ok(Frame::Heartbeat)
        );
    }

    #[test]
    fn test_decode_invalid() {
        assert_matches!(
            Frame::decode(&[b'\xC8'; Message::BYTESIZE]),
            Err(MessageError::NonUtf8(_))
        );
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(
            Message::truncate("abcde")
                .as_ref()
                .map(Message::text_as_str),
            Ok("abcde")
        );
    }

    #[test]
    fn test_truncate_long() {
        let text = "a".repeat(MAX_TEXT_LEN + 1);
        assert_eq!(
            Message::truncate(&text).as_ref().map(Message::text_as_str),
            Ok(&text[..MAX_TEXT_LEN])
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        let text = "Ⴀ".repeat(86);
        assert_eq!(
            Message::truncate(&text).as_ref().map(Message::text_as_str),
            Ok("Ⴀ".repeat(85).as_str())
        );

        assert_eq!('🦴'.len_utf8(), 4);
        let text = "🦴".repeat(64);
        assert_eq!(
            Message::truncate(&text).as_ref().map(Message::text_as_str),
            Ok("🦴".repeat(63).as_str())
        );
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(Message::truncate(""), Err(NonEmptyStringError::Empty));
    }
}
