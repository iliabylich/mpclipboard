use crate::{Buffer, Message, prelude::*};
use anyhow::anyhow;

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
    ) -> Completion<Message, anyhow::Error, ()> {
        let mut message = None;

        for &byte in bytes.as_slice() {
            if !self.buf.push(byte) {
                return Failed(anyhow!("malformed internal state"));
            }

            if let Some(full) = self.buf.as_full_array() {
                match Message::decode(full) {
                    Ok(m) => message = Some(m),
                    Err(err) => {
                        return Failed(err.context("failed to decode message in MessageReader"));
                    }
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
    use crate::{Message, NonEmptyInlineString, test_helpers::buffer};
    use anyhow::Result;

    #[test]
    fn test_receive_full() -> Result<()> {
        let mut reader = MessageReader::empty();

        let bytes = Message::new(NonEmptyInlineString::new("BOO")?)?.encode();
        let output = reader
            .received(buffer(&bytes)?)
            .expect_done("we've written a full message");
        assert_eq!(output.text_as_str(), "BOO");

        Ok(())
    }

    #[test]
    fn test_receive_step_by_step() -> Result<()> {
        let one = Message::new(NonEmptyInlineString::new("one")?)?;
        let two = Message::new(NonEmptyInlineString::new("twotwo")?)?;
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        reader
            .received(buffer(first)?)
            .expect_pending("only a part of the 1st message has been received");

        let message = reader
            .received(buffer(second)?)
            .expect_done("the rest of the 1st message has been received");
        assert_eq!(message, one);

        let message = reader
            .received(buffer(third)?)
            .expect_done("the rest of the 2nd message has been received");
        assert_eq!(message, two);

        Ok(())
    }
}
