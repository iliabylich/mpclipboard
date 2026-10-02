use crate::{Wants, message::Message};
use core::{cmp::Ordering, num::NonZeroUsize};

#[must_use]
#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub enum MessageWriter {
    Empty,

    Some {
        current: Writebuf<{ Message::BYTESIZE }>,
        next: Option<Writebuf<{ Message::BYTESIZE }>>,
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
            Self::Some { current, .. } => Some(current.remainder()),
        }
    }

    pub fn written(&mut self, n: NonZeroUsize) -> Result<(), MessageWriterError> {
        match self {
            Self::Empty => return Err(MessageWriterError::Empty),

            Self::Some { current, next } => {
                if current.written(n)? {
                    if let Some(next) = core::mem::take(next) {
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
        let item = Writebuf::new(data.encode());

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

#[derive(Clone, Copy)]
pub struct Writebuf<const N: usize> {
    buf: [u8; N],
    pos: usize,
}

impl<const N: usize> Writebuf<N> {
    pub(crate) const fn new(buf: [u8; N]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn remainder(&self) -> &[u8] {
        &self.buf[self.pos..]
    }

    pub(crate) fn written(&mut self, n: NonZeroUsize) -> Result<bool, MessageWriterError> {
        match self
            .pos
            .checked_add(n.get())
            .map(|newpos| (newpos, newpos.cmp(&N)))
        {
            Some((newpos, Ordering::Less)) => {
                self.pos = newpos;
                Ok(false)
            }
            Some((_, Ordering::Equal)) => {
                self.pos = 0;
                self.buf = [0; _];
                Ok(true)
            }
            None | Some((_, Ordering::Greater)) => Err(MessageWriterError::WrittenTooMuch {
                written: n.get(),
                remaining: self.remainder().len(),
            }),
        }
    }
}

impl<const N: usize> core::fmt::Debug for Writebuf<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Writebuf")
            .field("buf", &self.buf)
            .field("pos", &self.pos)
            .finish()
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
    use crate::{Message, NonEmptyInlineString, test_helpers::non_zero_usize};

    #[test]
    fn test_single() {
        let mut writer = MessageWriter::new();
        assert_eq!(writer.remainder(), None);

        let msg = Message::new(NonEmptyInlineString::const_new("FOO"));
        let encoded = msg.encode();
        writer.push(&msg);
        assert_eq!(writer.remainder(), Some(encoded.as_slice()));
        assert_eq!(writer.written(non_zero_usize(100)), Ok(()));
        assert_eq!(writer.remainder(), encoded.get(100..));
        assert_eq!(
            writer.written(non_zero_usize(Message::BYTESIZE - 100)),
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
        assert_eq!(writer.written(non_zero_usize(Message::BYTESIZE)), Ok(()));
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
            writer.written(non_zero_usize(Message::BYTESIZE)),
            Err(MessageWriterError::WrittenTooMuch {
                written: Message::BYTESIZE,
                remaining: Message::BYTESIZE - 100,
            })
        );
        assert_eq!(
            writer.remainder().map(<[u8]>::len),
            Some(Message::BYTESIZE - 100)
        );
    }
}
