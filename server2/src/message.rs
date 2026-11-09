use crate::fixed_str::FixedStr;
use core::num::NonZeroUsize;

const MAX_TEXT_LEN: usize = 255;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Message {
    text: FixedStr<MAX_TEXT_LEN>,
}

impl Message {
    pub(crate) const BYTESIZE: usize = {
        let size = size_of::<u8>() + MAX_TEXT_LEN;
        assert!(size == 256);
        size
    };

    pub(crate) fn text(&self) -> &str {
        self.text.as_str()
    }

    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let text = self.text.as_bytes();
        let len = u8::try_from(text.len())
            .unwrap_or_else(|_| unreachable!("text never exceeds MAX_TEXT_LEN"));

        let mut buf = [0; Self::BYTESIZE];
        let [len_slot, rest @ ..] = &mut buf;
        *len_slot = len;
        rest[..text.len()].copy_from_slice(text);
        buf
    }
}

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Text(\"{}\")", self.text())
    }
}

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Frame {
    Message(Message),
    Heartbeat,
}

impl Frame {
    pub(crate) fn decode(buf: &[u8; Message::BYTESIZE]) -> Result<Self, ()> {
        let [len, text @ ..] = buf;

        let Some(len) = NonZeroUsize::new(usize::from(*len)) else {
            return Ok(Self::Heartbeat);
        };
        let text = match core::str::from_utf8(&text[..len.get()]) {
            Ok(text) => text,
            Err(err) => {
                log::error!("non-utf8 message text: {err}");
                return Err(());
            }
        };
        let text = FixedStr::new(text)
            .unwrap_or_else(|_| unreachable!("len is non-zero and fits into a u8"));

        Ok(Self::Message(Message { text }))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{Frame, MAX_TEXT_LEN, Message};
    use std::string::String;

    pub(crate) fn frame(text: &str) -> [u8; Message::BYTESIZE] {
        let mut buf = [0; Message::BYTESIZE];
        buf[0] = u8::try_from(text.len()).unwrap();
        buf[1..=text.len()].copy_from_slice(text.as_bytes());
        buf
    }

    pub(crate) fn text(frame: Frame) -> String {
        match frame {
            Frame::Message(message) => String::from(message.text()),
            Frame::Heartbeat => panic!("expected a message, got a heartbeat"),
        }
    }

    pub(crate) fn message(text: &str) -> Message {
        match Frame::decode(&frame(text)).unwrap() {
            Frame::Message(message) => message,
            Frame::Heartbeat => panic!("expected a message, got a heartbeat"),
        }
    }

    #[test]
    fn test_encode_decode() {
        let buf = frame("aaaaaaaaaa");
        let decoded = Frame::decode(&buf).unwrap();
        assert_eq!(text(decoded), "aaaaaaaaaa");
        let Frame::Message(message) = decoded else {
            unreachable!()
        };
        assert_eq!(message.encode(), buf);
    }

    #[test]
    fn test_encode_decode_max_len() {
        let max = "a".repeat(MAX_TEXT_LEN);
        let buf = frame(&max);
        let decoded = Frame::decode(&buf).unwrap();
        assert_eq!(text(decoded), max);
        let Frame::Message(message) = decoded else {
            unreachable!()
        };
        assert_eq!(message.encode(), buf);
    }

    #[test]
    fn test_heartbeat() {
        assert_eq!(Frame::decode(&[0; Message::BYTESIZE]), Ok(Frame::Heartbeat));
    }

    #[test]
    fn test_heartbeat_ignores_padding() {
        let mut buf = [b'x'; Message::BYTESIZE];
        buf[0] = 0;
        assert_eq!(Frame::decode(&buf), Ok(Frame::Heartbeat));
    }

    #[test]
    fn test_decode_invalid() {
        assert_eq!(Frame::decode(&[b'\xC8'; Message::BYTESIZE]), Err(()));
    }
}
