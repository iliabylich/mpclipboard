use crate::{
    Buffer, Wants,
    message::{Message, MessageSize},
};
use core::num::NonZeroUsize;

#[must_use]
#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum MessageWriter {
    Empty,

    Some {
        current: Buffer<MessageSize>,
        next: Option<Buffer<MessageSize>>,
    },
}

impl MessageWriter {
    pub const fn new() -> Self {
        Self::Empty
    }

    #[must_use]
    pub fn remainder(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Some { current, .. } => Some(current.as_slice()),
        }
    }

    pub fn written(&mut self, n: NonZeroUsize) -> Result<(), MessageWriterError> {
        match self {
            Self::Empty => return Err(MessageWriterError::Empty),

            Self::Some { current, next } => {
                current.drop_n_front_bytes(n.get()).map_err(|remaining| {
                    MessageWriterError::WrittenTooMuch {
                        written: n.get(),
                        remaining,
                    }
                })?;

                if current.as_slice().is_empty() {
                    if let Some(next) = next.take() {
                        *current = next;
                    } else {
                        *self = Self::new();
                    }
                }
            }
        }

        Ok(())
    }

    pub fn push(&mut self, data: &Message) {
        let item = Buffer::new(data.encode());

        match self {
            Self::Empty => {
                *self = Self::Some {
                    current: item,
                    next: None,
                }
            }
            Self::Some { next, .. } => *next = Some(item),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remainder().is_none()
    }

    #[must_use]
    pub fn wants(&self) -> Option<Wants> {
        if self.is_empty() {
            None
        } else {
            Some(Wants::Write)
        }
    }
}

impl Default for MessageWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageWriterError {
    #[error("written() called on an empty MessageWriter")]
    Empty,
    #[error("written() reported {written} bytes, but only {remaining} bytes remained")]
    WrittenTooMuch { written: usize, remaining: usize },
}

#[cfg(test)]
mod tests {
    use super::{MessageWriter, MessageWriterError};
    use crate::{
        Message, NonEmptyInlineString, message::MessageSize, test_helpers::non_zero_usize,
    };
    use typenum::Unsigned;

    #[test]
    fn test_single() {
        let mut writer = MessageWriter::new();
        assert_eq!(writer.remainder(), None);

        let msg = Message::new(NonEmptyInlineString::const_new("FOO"));
        let encoded = msg.encode();
        writer.push(&msg);
        assert_eq!(writer.remainder(), Some(encoded.as_slice()));
        assert_eq!(writer.written(non_zero_usize(100)), Ok(()));
        assert_eq!(writer.remainder(), Some(&encoded.as_slice()[100..]));
        assert_eq!(
            writer.written(non_zero_usize(MessageSize::USIZE - 100)),
            Ok(())
        );
        assert_eq!(writer.remainder(), None);
    }

    #[test]
    fn test_fixed_size_queue_like_with_tail_replacement() {
        let mut writer = MessageWriter::new();

        let msg1 = Message::new(NonEmptyInlineString::const_new("msg1"));
        writer.push(&msg1);
        let msg2 = Message::new(NonEmptyInlineString::const_new("msg2"));
        writer.push(&msg2);
        let msg3 = Message::new(NonEmptyInlineString::const_new("msg3"));
        writer.push(&msg3);

        assert_eq!(writer.remainder(), Some(msg1.encode().as_slice()));
        assert_eq!(writer.written(non_zero_usize(MessageSize::USIZE)), Ok(()));
        assert_eq!(writer.remainder(), Some(msg3.encode().as_slice()));
    }

    #[test]
    fn test_errors() {
        let mut writer = MessageWriter::new();
        assert_eq!(
            writer.written(non_zero_usize(1)),
            Err(MessageWriterError::Empty)
        );

        writer.push(&Message::new(NonEmptyInlineString::const_new("FOO")));
        assert_eq!(writer.written(non_zero_usize(100)), Ok(()));
        assert_eq!(
            writer.written(non_zero_usize(MessageSize::USIZE)),
            Err(MessageWriterError::WrittenTooMuch {
                written: MessageSize::USIZE,
                remaining: MessageSize::USIZE - 100,
            })
        );
        assert_eq!(
            writer.remainder().map(<[u8]>::len),
            Some(MessageSize::USIZE - 100)
        );
    }
}
