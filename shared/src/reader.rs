use crate::{readbuf::Readbuf, trace};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Reader<const N: usize> {
    readbuf: Readbuf<N>,
}

impl<const N: usize> Reader<N> {
    pub(crate) const fn new() -> Self {
        Self {
            readbuf: Readbuf::new(),
        }
    }

    pub(crate) fn new_with_data(data: &[u8]) -> (Self, Option<[u8; N]>) {
        assert!(data.len() <= N);

        if let Ok(buf) = data.try_into() {
            return (Self::new(), Some(buf));
        }

        (
            Self {
                readbuf: Readbuf::new_with_data(data),
            },
            None,
        )
    }

    pub(crate) fn received(&mut self, data: &[u8]) -> (usize, Option<[u8; N]>) {
        trace!("received {} bytes", data.len());
        self.readbuf.received(data)
    }

    pub(crate) fn remaining(&self) -> usize {
        self.readbuf.remaining()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NonEmptyInlineString, message::Message};

    fn message(text: &str) -> Message {
        Message::new(NonEmptyInlineString::new(text).unwrap_or_else(|_| unreachable!()))
    }

    fn encoded(message: Message) -> [u8; Message::BYTESIZE] {
        message.encode()
    }

    #[test]
    fn complete_leftover_is_returned_immediately() {
        let message = message("hello");
        let buf = encoded(message);

        let (_, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(&buf);

        match res {
            Some(data) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn partial_leftover_is_preserved() {
        let message = message("hello");
        let buf = encoded(message);
        let (partial, rest) = buf.split_at(Message::BYTESIZE - 1);
        let (mut reader, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(partial);
        assert!(res.is_none());
        let (consumed, result) = reader.received(rest);
        assert_eq!(consumed, rest.len());
        match result {
            Some(data) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn complete_leftover_is_not_decoded() {
        let buf = [0; Message::BYTESIZE];
        let (_, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(&buf);

        match res {
            Some(data) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
