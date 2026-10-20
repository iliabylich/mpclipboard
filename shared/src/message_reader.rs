use crate::{Buffer, Message, MessageError, PushResult, message::Frame, prelude::*};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct MessageReader {
    buf: Buffer<{ Message::BYTESIZE - 1 }>,
}

impl MessageReader {
    pub const fn empty() -> Self {
        Self {
            buf: Buffer::empty(),
        }
    }

    pub const fn new(partial: Buffer<{ Message::BYTESIZE - 1 }>) -> Self {
        Self { buf: partial }
    }

    pub fn received(&mut self, bytes: &[u8]) -> Result<Completion<Message, ()>, MessageError> {
        let mut message = None;

        for &byte in bytes {
            if let PushResult::Full(buffered) = self.buf.push(byte) {
                if let Frame::Message(decoded) = Frame::decode(&concat(buffered, byte))? {
                    message = Some(decoded);
                }
                self.buf.clear();
            }
        }

        if let Some(message) = message {
            Ok(Done(message))
        } else {
            Ok(Pending(()))
        }
    }
}

impl Default for MessageReader {
    fn default() -> Self {
        Self::empty()
    }
}

fn concat<const N: usize, const M: usize>(head: &[u8; N], last: u8) -> [u8; M] {
    const { assert!(N + 1 == M, "M must be N + 1") };

    let mut out = [0; M];
    for (dst, src) in out.iter_mut().zip(head.iter().copied().chain([last])) {
        *dst = src;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::MessageReader;
    use crate::{Message, MessageError, NonEmptyInlineString, message::Frame, prelude::*};
    use core::assert_matches;

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::empty();

        let message = Message::new(NonEmptyInlineString::const_new("BOO"));
        assert_eq!(reader.received(&message.encode()), Ok(Done(message)));
    }

    #[test]
    fn test_receive_step_by_step() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let two = Message::new(NonEmptyInlineString::const_new("twotwo"));
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(first), Ok(Pending(())));
        assert_eq!(reader.received(second), Ok(Done(one)));
        assert_eq!(reader.received(third), Ok(Done(two)));
    }

    #[test]
    fn test_receive_many_at_once() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let two = Message::new(NonEmptyInlineString::const_new("twotwo"));
        let three = Message::new(NonEmptyInlineString::const_new("three"));
        let stream = [one.encode(), two.encode(), three.encode()].concat();

        let (head, tail) = stream.split_at(Message::BYTESIZE * 2 + 10);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(head), Ok(Done(two)));
        assert_eq!(reader.received(tail), Ok(Done(three)));
    }

    #[test]
    fn test_receive_heartbeat() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let stream = [Frame::Heartbeat.encode(), one.encode()].concat();
        let (heartbeat, message) = stream.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(heartbeat), Ok(Pending(())));
        assert_eq!(reader.received(message), Ok(Done(one)));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::empty();

        assert_matches!(
            reader.received(&[b'\xC8'; Message::BYTESIZE]),
            Err(MessageError::NonUtf8(_))
        );
    }
}
