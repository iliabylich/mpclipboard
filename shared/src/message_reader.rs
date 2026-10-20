use crate::{Message, MessageDecodeError, message::Frame, prelude::*};
use bytes::BytesMut;

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct MessageReader {
    buf: BytesMut,
}

impl MessageReader {
    pub fn new(partial: &[u8]) -> Self {
        Self {
            buf: BytesMut::from(partial),
        }
    }

    pub fn received(
        &mut self,
        bytes: &[u8],
    ) -> Result<Completion<Message, ()>, MessageDecodeError> {
        self.buf.extend_from_slice(bytes);
        let mut message = None;

        while self.buf.len() >= Message::BYTESIZE {
            let frame = self.buf.split_to(Message::BYTESIZE);
            let frame = frame
                .as_array::<{ Message::BYTESIZE }>()
                .unwrap_or_else(|| unreachable!("frame is split at Message::BYTESIZE"));
            if let Frame::Message(decoded) = Frame::decode(frame)? {
                message = Some(decoded);
            }
        }

        if let Some(message) = message {
            Ok(Done(message))
        } else {
            Ok(Pending(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageReader;
    use crate::{Message, MessageDecodeError, message::Frame, prelude::*};
    use core::assert_matches;

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::default();

        let message = Message::truncate("BOO").unwrap();
        assert_eq!(reader.received(&message.encode()), Ok(Done(message)));
    }

    #[test]
    fn test_receive_step_by_step() {
        let one = Message::truncate("one").unwrap();
        let two = Message::truncate("twotwo").unwrap();
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::default();

        assert_eq!(reader.received(first), Ok(Pending(())));
        assert_eq!(reader.received(second), Ok(Done(one)));
        assert_eq!(reader.received(third), Ok(Done(two)));
    }

    #[test]
    fn test_receive_many_at_once() {
        let one = Message::truncate("one").unwrap();
        let two = Message::truncate("twotwo").unwrap();
        let three = Message::truncate("three").unwrap();
        let stream = [one.encode(), two.encode(), three.encode()].concat();

        let (head, tail) = stream.split_at(Message::BYTESIZE * 2 + 10);

        let mut reader = MessageReader::default();

        assert_eq!(reader.received(head), Ok(Done(two)));
        assert_eq!(reader.received(tail), Ok(Done(three)));
    }

    #[test]
    fn test_receive_heartbeat() {
        let one = Message::truncate("one").unwrap();
        let stream = [Frame::Heartbeat.encode(), one.encode()].concat();
        let (heartbeat, message) = stream.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::default();

        assert_eq!(reader.received(heartbeat), Ok(Pending(())));
        assert_eq!(reader.received(message), Ok(Done(one)));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::default();

        assert_matches!(
            reader.received(&[b'\xC8'; Message::BYTESIZE]),
            Err(MessageDecodeError::NonUtf8(_))
        );
    }
}
