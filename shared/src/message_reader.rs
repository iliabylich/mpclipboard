use crate::{Buffer, Message, MessageError, prelude::*};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct MessageReader {
    buf: Buffer<{ Message::BYTESIZE }>,
}

impl MessageReader {
    pub const fn empty() -> Self {
        Self { buf: Buffer::new() }
    }

    pub fn new(partial: Buffer<{ Message::BYTESIZE - 1 }>) -> Self {
        let Some(buf) = Buffer::from_slice(partial.as_slice()) else {
            unreachable!("partial message is always shorter than a message");
        };
        Self { buf }
    }

    pub fn received(
        &mut self,
        bytes: Buffer<{ Message::BYTESIZE }>,
    ) -> Completion<Message, MessageError, ()> {
        let mut message = None;

        for &byte in bytes.as_slice() {
            if !self.buf.push(byte) {
                unreachable!("buf is cleared as soon as it's full");
            }

            if let Some(full) = self.buf.as_full_array() {
                match Message::decode(full) {
                    Ok(m) => message = Some(m),
                    Err(err) => return Failed(err),
                }
                self.buf.clear();
            }
        }

        if let Some(message) = message {
            Done(message)
        } else {
            Pending(())
        }
    }
}

impl Default for MessageReader {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::MessageReader;
    use crate::{Message, MessageError, NonEmptyInlineString, prelude::*, test_helpers::buffer};

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::empty();

        let message = Message::new(NonEmptyInlineString::const_new("BOO"));
        assert_eq!(reader.received(buffer(&message.encode())), Done(message));
    }

    #[test]
    fn test_receive_step_by_step() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let two = Message::new(NonEmptyInlineString::const_new("twotwo"));
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(buffer(first)), Pending(()));
        assert_eq!(reader.received(buffer(second)), Done(one));
        assert_eq!(reader.received(buffer(third)), Done(two));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::empty();

        assert_eq!(
            reader.received(buffer(&[0; Message::BYTESIZE])),
            Failed(MessageError::Empty)
        );
    }
}
