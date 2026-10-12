use crate::{Buffer, Message, MessageError, PushResult, prelude::*};

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

    pub fn received(
        &mut self,
        bytes: Buffer<{ Message::BYTESIZE }>,
    ) -> Result<Completion<Message, ()>, MessageError> {
        let mut message = None;

        for &byte in bytes.as_slice() {
            if let PushResult::Full(buffered) = self.buf.push(byte) {
                message = Some(Message::decode(&concat(buffered, byte))?);
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
    use crate::{Message, MessageError, NonEmptyInlineString, prelude::*, test_helpers::buffer};

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::empty();

        let message = Message::new(NonEmptyInlineString::const_new("BOO"));
        assert_eq!(
            reader.received(buffer(&message.encode())),
            Ok(Done(message))
        );
    }

    #[test]
    fn test_receive_step_by_step() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let two = Message::new(NonEmptyInlineString::const_new("twotwo"));
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(buffer(first)), Ok(Pending(())));
        assert_eq!(reader.received(buffer(second)), Ok(Done(one)));
        assert_eq!(reader.received(buffer(third)), Ok(Done(two)));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::empty();

        assert_eq!(
            reader.received(buffer(&[0; Message::BYTESIZE])),
            Err(MessageError::Empty)
        );
    }
}
