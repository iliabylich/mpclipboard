use crate::message::{Frame, Message};

pub(crate) struct MessageReader {
    buf: [u8; Message::BYTESIZE],
    filled: usize,
}

impl MessageReader {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; Message::BYTESIZE],
            filled: 0,
        }
    }

    pub(crate) fn received(&mut self, mut bytes: &[u8]) -> Result<Option<Message>, ()> {
        let mut message = None;

        while !bytes.is_empty() {
            let (head, tail) = bytes.split_at(bytes.len().min(Message::BYTESIZE - self.filled));
            self.buf[self.filled..self.filled + head.len()].copy_from_slice(head);
            self.filled += head.len();
            bytes = tail;

            if self.filled == Message::BYTESIZE {
                self.filled = 0;
                if let Frame::Message(decoded) = Frame::decode(&self.buf)? {
                    message = Some(decoded);
                }
            }
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::MessageReader;
    use crate::message::{
        Frame, Message,
        tests::{frame, text},
    };
    use std::{option::Option, string::String};

    fn received(reader: &mut MessageReader, bytes: &[u8]) -> Result<Option<String>, ()> {
        let message = reader.received(bytes)?;
        Ok(message.map(|message| text(Frame::Message(message))))
    }

    #[test]
    fn test_receive_full() {
        let mut reader = MessageReader::new();
        assert_eq!(received(&mut reader, &frame("BOO")), Ok(Some("BOO".into())));
    }

    #[test]
    fn test_receive_step_by_step() {
        let stream = [frame("one"), frame("twotwo")].concat();

        let (first, rest) = stream.split_at(100);
        let (second, third) = rest.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::new();
        assert_eq!(received(&mut reader, first), Ok(None));
        assert_eq!(received(&mut reader, second), Ok(Some("one".into())));
        assert_eq!(received(&mut reader, third), Ok(Some("twotwo".into())));
    }

    #[test]
    fn test_receive_many_at_once() {
        let stream = [frame("one"), frame("twotwo"), frame("three")].concat();

        let (head, tail) = stream.split_at(Message::BYTESIZE * 2 + 10);

        let mut reader = MessageReader::new();
        assert_eq!(received(&mut reader, head), Ok(Some("twotwo".into())));
        assert_eq!(received(&mut reader, tail), Ok(Some("three".into())));
    }

    #[test]
    fn test_receive_byte_by_byte() {
        let stream = [frame("one"), frame("two")].concat();

        let mut reader = MessageReader::new();
        let mut out = std::vec::Vec::new();
        for byte in &stream {
            if let Some(message) = received(&mut reader, &[*byte]).unwrap() {
                out.push(message);
            }
        }
        assert_eq!(out, ["one", "two"]);
    }

    #[test]
    fn test_receive_heartbeat() {
        let stream = [[0; Message::BYTESIZE], frame("one")].concat();
        let (heartbeat, message) = stream.split_at(Message::BYTESIZE);

        let mut reader = MessageReader::new();
        assert_eq!(received(&mut reader, heartbeat), Ok(None));
        assert_eq!(received(&mut reader, message), Ok(Some("one".into())));
    }

    #[test]
    fn test_receive_invalid() {
        let mut reader = MessageReader::new();
        assert_eq!(
            received(&mut reader, &[b'\xC8'; Message::BYTESIZE]),
            Err(())
        );
    }
}
