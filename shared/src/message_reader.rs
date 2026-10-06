use crate::{Buffer, Message, MessageError, PushResult, message::MessageSize, prelude::*};
use generic_array::sequence::Lengthen;
use typenum::{U1, op};

#[must_use]
#[derive(Debug, Clone)]
pub struct MessageReader {
    buf: Buffer<op!(MessageSize - U1)>,
}

impl MessageReader {
    pub fn empty() -> Self {
        Self {
            buf: Buffer::empty(),
        }
    }

    pub const fn new(partial: Buffer<op!(MessageSize - U1)>) -> Self {
        Self { buf: partial }
    }

    pub fn received(
        &mut self,
        bytes: &Buffer<MessageSize>,
    ) -> Result<Completion<Message, ()>, MessageError> {
        let mut message = None;

        for &byte in bytes.as_slice() {
            if let PushResult::Full(buffered) = self.buf.push(byte) {
                message = Some(Message::decode(buffered.append(byte))?);
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

#[cfg(test)]
mod tests {
    use super::MessageReader;
    use crate::{
        Message, MessageError, NonEmptyInlineString, message::MessageSize, prelude::*,
        test_helpers::buffer,
    };
    use typenum::Unsigned;

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::empty();

        let message = Message::new(NonEmptyInlineString::const_new("BOO"));
        assert_eq!(
            reader.received(&buffer(&message.encode())),
            Ok(Done(message))
        );
    }

    #[test]
    fn test_receive_step_by_step() {
        let one = Message::new(NonEmptyInlineString::const_new("one"));
        let two = Message::new(NonEmptyInlineString::const_new("twotwo"));
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(MessageSize::USIZE);

        let mut reader = MessageReader::empty();

        assert_eq!(reader.received(&buffer(first)), Ok(Pending(())));
        assert_eq!(reader.received(&buffer(second)), Ok(Done(one)));
        assert_eq!(reader.received(&buffer(third)), Ok(Done(two)));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::empty();

        assert_eq!(
            reader.received(&buffer(&[0; MessageSize::USIZE])),
            Err(MessageError::Empty)
        );
    }
}
