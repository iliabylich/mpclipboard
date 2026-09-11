use crate::{Message, MessageDecodeError};

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
        bytes: &[u8; Message::BYTESIZE],
        len: usize,
    ) -> Result<Option<Message>, MessageDecodeError> {
        if self.pos >= Message::BYTESIZE {
            unreachable!("malformed state")
        }

        let mut message = None;
        let bytes = bytes
            .get(..len)
            .unwrap_or_else(|| unreachable!("malformed buffer"));

        for &byte in bytes {
            let slot = self
                .buf
                .get_mut(self.pos)
                .unwrap_or_else(|| unreachable!("malformed internal state"));
            *slot = byte;
            self.pos = self
                .pos
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("buffer pos overflow"));

            if self.pos == Message::BYTESIZE {
                message = Some(Message::decode(&self.buf)?);
                self.buf = [0; _];
                self.pos = 0;
            }
        }

        Ok(message)
    }

    #[must_use]
    pub fn bytes_needed(&self) -> usize {
        Message::BYTESIZE
            .checked_sub(self.pos)
            .unwrap_or_else(|| unreachable!("malformed internal state"))
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
    use crate::{Message, NonEmptyInlineString};

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::empty();
        assert_eq!(reader.bytes_needed(), Message::BYTESIZE);

        let bytes: [u8; Message::BYTESIZE] =
            Message::new(NonEmptyInlineString::new("BOO").unwrap()).encode();
        let output = reader.received(&bytes, Message::BYTESIZE).unwrap().unwrap();
        assert_eq!(output.text_as_str(), "BOO");
    }

    #[test]
    fn test_receive_step_by_step() {
        let mut reader = MessageReader::empty();
        assert_eq!(reader.bytes_needed(), Message::BYTESIZE);

        // ab
        let one: [u8; Message::BYTESIZE] =
            Message::new(NonEmptyInlineString::new("one").unwrap()).encode();
        // cd
        let two: [u8; Message::BYTESIZE] =
            Message::new(NonEmptyInlineString::new("twotwo").unwrap()).encode();

        // write "a"
        let mut buf1 = [0; Message::BYTESIZE];
        buf1[..100].copy_from_slice(&one[..100]);
        assert_eq!(reader.received(&buf1, 100), Ok(None));

        // write "bc"
        let mut buf2 = [0; Message::BYTESIZE];
        buf2[..Message::BYTESIZE - 100].copy_from_slice(&one[100..]);
        buf2[Message::BYTESIZE - 100..].copy_from_slice(&two[..100]);
        let message1 = reader.received(&buf2, Message::BYTESIZE).unwrap().unwrap();
        assert_eq!(message1.text_as_str(), "one");

        // write "d"
        let mut buf3 = [0; Message::BYTESIZE];
        buf3[..Message::BYTESIZE - 100].copy_from_slice(&two[100..]);
        let message2 = reader
            .received(&buf3, Message::BYTESIZE - 100)
            .unwrap()
            .unwrap();
        assert_eq!(message2.text_as_str(), "twotwo");
    }
}
