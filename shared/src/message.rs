use crate::FixedStr;
use core::str::Utf8Error;

const MAX_TEXT_LENGTH: usize = 255;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Message {
    text: FixedStr<MAX_TEXT_LENGTH>,
}

impl Message {
    pub const BYTESIZE: usize = {
        let size = size_of::<u8>() + MAX_TEXT_LENGTH;
        assert!(size == 256);
        size
    };

    pub fn truncate(text: &str) -> Option<Self> {
        let text = &text[..text.floor_char_boundary(MAX_TEXT_LENGTH)];
        let text = FixedStr::new(text).ok()?;
        Some(Self { text })
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn encode(&self) -> [u8; Self::BYTESIZE] {
        let text = self.text.as_bytes();
        let len = u8::try_from(text.len())
            .unwrap_or_else(|_| unreachable!("text never exceeds MAX_TEXT_LEN"));

        let mut buf = [0; Self::BYTESIZE];
        let [len_slot, rest @ ..] = &mut buf;
        *len_slot = len;
        rest[..text.len()].copy_from_slice(text);
        buf
    }

    pub fn decode(buf: &[u8; Self::BYTESIZE]) -> Result<Self, MessageDecodeError> {
        let [len, text @ ..] = buf;

        if *len == 0 {
            return Err(MessageDecodeError::Empty);
        }
        let text = core::str::from_utf8(&text[..usize::from(*len)])?;
        let text = FixedStr::new(text)
            .unwrap_or_else(|_| unreachable!("len is non-zero and fits into a u8"));

        Ok(Self { text })
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text(\"{}\")", self.text())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDecodeError {
    Empty,
    NonUtf8(Utf8Error),
}

impl From<Utf8Error> for MessageDecodeError {
    fn from(err: Utf8Error) -> Self {
        Self::NonUtf8(err)
    }
}

impl core::fmt::Display for MessageDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("message is empty"),
            Self::NonUtf8(err) => write!(f, "message is not valid utf-8: {err}"),
        }
    }
}

impl core::error::Error for MessageDecodeError {}

#[cfg(test)]
mod tests {
    use super::{MAX_TEXT_LENGTH, Message, MessageDecodeError};
    use std::format;

    fn frame(text: &str) -> [u8; Message::BYTESIZE] {
        let mut buf = [0; Message::BYTESIZE];
        buf[0] = u8::try_from(text.len()).unwrap();
        buf[1..=text.len()].copy_from_slice(text.as_bytes());
        buf
    }

    #[test]
    fn test_truncate() {
        assert_eq!(Message::truncate("abc").map(|m| m.text().len()), Some(3));
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(Message::truncate(""), None);
    }

    #[test]
    fn test_truncate_long() {
        let text = "a".repeat(MAX_TEXT_LENGTH + 10);
        let message = Message::truncate(&text).unwrap();
        assert_eq!(message.text(), &text[..MAX_TEXT_LENGTH]);
    }

    #[test]
    fn test_truncate_at_char_boundary() {
        let text = format!("{}Ⴀ", "a".repeat(MAX_TEXT_LENGTH - 1));
        let message = Message::truncate(&text).unwrap();
        assert_eq!(message.text(), "a".repeat(MAX_TEXT_LENGTH - 1));
    }

    #[test]
    fn test_encode() {
        let encoded = Message::truncate("abc").unwrap().encode();
        assert_eq!(encoded[..4], [3, b'a', b'b', b'c']);
        assert!(encoded[4..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn test_encode_decode() {
        let buf = frame("aaaaaaaaaa");
        let message = Message::decode(&buf).unwrap();
        assert_eq!(message.text(), "aaaaaaaaaa");
        assert_eq!(message.encode(), buf);
    }

    #[test]
    fn test_encode_decode_max_len() {
        let max = "a".repeat(MAX_TEXT_LENGTH);
        let buf = frame(&max);
        let message = Message::decode(&buf).unwrap();
        assert_eq!(message.text(), max);
        assert_eq!(message.encode(), buf);
    }

    #[test]
    fn test_decode_empty() {
        assert_eq!(
            Message::decode(&[0; Message::BYTESIZE]),
            Err(MessageDecodeError::Empty)
        );
    }

    #[test]
    fn test_decode_empty_ignores_padding() {
        let mut buf = [b'x'; Message::BYTESIZE];
        buf[0] = 0;
        assert_eq!(Message::decode(&buf), Err(MessageDecodeError::Empty));
    }

    #[test]
    fn test_decode_invalid() {
        assert!(matches!(
            Message::decode(&[b'\xC8'; Message::BYTESIZE]),
            Err(MessageDecodeError::NonUtf8(_))
        ));
    }
}
