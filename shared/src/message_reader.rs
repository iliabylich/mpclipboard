use crate::{Buffer, Message, prelude::*};
use anyhow::anyhow;
use core::num::NonZeroUsize;

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
        bytes: [u8; Message::BYTESIZE],
        len: NonZeroUsize,
    ) -> Completion<Message, anyhow::Error, ()> {
        let mut message = None;
        let Some(bytes) = bytes.get(..len.get()) else {
            return Failed(anyhow!("malformed buffer"));
        };

        for &byte in bytes {
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
    use crate::{Message, NonEmptyInlineString, test_helpers::non_zero_usize};
    use anyhow::{Context, Result};
    use core::num::NonZeroUsize;

    #[test]
    fn test_receive_full() -> Result<()> {
        let mut reader = MessageReader::empty();

        let bytes = Message::new(NonEmptyInlineString::new("BOO")?)?.encode();
        let output = reader
            .received(bytes, non_zero_usize(Message::BYTESIZE)?)
            .expect_done("we've written a full message");
        assert_eq!(output.text_as_str(), "BOO");

        Ok(())
    }

    fn chunk(bytes: &[u8]) -> Result<([u8; Message::BYTESIZE], NonZeroUsize)> {
        let mut buf = [0; Message::BYTESIZE];
        buf.get_mut(..bytes.len())
            .context("chunk is longer than a message")?
            .copy_from_slice(bytes);
        Ok((buf, non_zero_usize(bytes.len())?))
    }

    #[test]
    fn test_receive_step_by_step() -> Result<()> {
        let one = Message::new(NonEmptyInlineString::new("one")?)?;
        let two = Message::new(NonEmptyInlineString::new("twotwo")?)?;
        let stream = [one.encode(), two.encode()].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::empty();

        let (buf, len) = chunk(first)?;
        reader
            .received(buf, len)
            .expect_pending("only a part of the 1st message has been received");

        let (buf, len) = chunk(second)?;
        let message = reader
            .received(buf, len)
            .expect_done("the rest of the 1st message has been received");
        assert_eq!(message, one);

        let (buf, len) = chunk(third)?;
        let message = reader
            .received(buf, len)
            .expect_done("the rest of the 2nd message has been received");
        assert_eq!(message, two);

        Ok(())
    }
}
