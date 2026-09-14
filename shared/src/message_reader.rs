use crate::{Message, prelude::*};
use anyhow::anyhow;
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct MessageReader {
    buf: [u8; Message::BYTESIZE],
    pos: usize,
}

impl MessageReader {
    pub const fn empty() -> Self {
        Self {
            buf: [0; _],
            pos: 0,
        }
    }

    pub const fn new(buf: [u8; Message::BYTESIZE], pos: usize) -> Self {
        Self { buf, pos }
    }

    pub fn received(
        &mut self,
        bytes: [u8; Message::BYTESIZE],
        len: NonZeroUsize,
    ) -> Completion<Message, anyhow::Error, ()> {
        if self.pos >= Message::BYTESIZE {
            return Failed(anyhow!("malformed state"));
        }

        let mut message = None;
        let Some(bytes) = bytes.get(..len.get()) else {
            return Failed(anyhow!("malformed buffer"));
        };

        for &byte in bytes {
            let Some(slot) = self.buf.get_mut(self.pos) else {
                return Failed(anyhow!("malformed internal state"));
            };
            *slot = byte;

            let Some(nextpos) = self.pos.checked_add(1) else {
                return Failed(anyhow!("buffer pos overflow"));
            };
            self.pos = nextpos;

            if self.pos == Message::BYTESIZE {
                match Message::decode(&self.buf) {
                    Ok(m) => message = Some(m),
                    Err(err) => {
                        return Failed(err.context("failed to decode message in MessageReader"));
                    }
                }
                self.buf = [0; _];
                self.pos = 0;
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
    use anyhow::Result;
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

    #[test]
    fn test_receive_step_by_step() -> Result<()> {
        let mut reader = MessageReader::empty();

        // ab
        let one: [u8; Message::BYTESIZE] =
            Message::new(NonEmptyInlineString::new("one")?)?.encode();
        // cd
        let two: [u8; Message::BYTESIZE] =
            Message::new(NonEmptyInlineString::new("twotwo")?)?.encode();

        // write "a"
        let mut buf1 = [0; Message::BYTESIZE];
        buf1[..100].copy_from_slice(&one[..100]);
        reader
            .received(buf1, NonZeroUsize::new(100).expect("literal argument"))
            .expect_pending("only 'a' has been written so far");

        // write "bc"
        let mut buf2 = [0; Message::BYTESIZE];
        buf2[..Message::BYTESIZE - 100].copy_from_slice(&one[100..]);
        buf2[Message::BYTESIZE - 100..].copy_from_slice(&two[..100]);
        let message1 = reader
            .received(
                buf2,
                NonZeroUsize::new(Message::BYTESIZE).expect("literal argument"),
            )
            .expect_done("we've written 'a' -> 'bc', so the first message is there");
        assert_eq!(message1.text_as_str(), "one");

        // write "d"
        let mut buf3 = [0; Message::BYTESIZE];
        buf3[..Message::BYTESIZE - 100].copy_from_slice(&two[100..]);
        let message2 = reader
            .received(
                buf3,
                NonZeroUsize::new(Message::BYTESIZE - 100).expect("literal argument"),
            )
            .expect_done("we've finished writing 'cd', so the 2nd message is also there now");
        assert_eq!(message2.text_as_str(), "twotwo");

        Ok(())
    }
}
