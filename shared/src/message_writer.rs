use crate::{Wants, message::Message};
use anyhow::{Context, Result, bail};
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

    pub fn written(&mut self, n: NonZeroUsize) -> Result<()> {
        match self {
            Self::Empty => bail!("empty buffer never wants to write"),

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

    pub(crate) fn written(&mut self, n: NonZeroUsize) -> Result<bool> {
        self.pos = self
            .pos
            .checked_add(n.get())
            .context("overflow: n is too large")?;

        match (self.pos).cmp(&N) {
            Ordering::Less => Ok(false),
            Ordering::Equal => {
                self.pos = 0;
                self.buf = [0; _];
                Ok(true)
            }
            Ordering::Greater => bail!("buffer overflow"),
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

#[cfg(test)]
mod tests {
    use super::MessageWriter;
    use crate::{Message, NonEmptyInlineString, test_helpers::non_zero_usize};
    use anyhow::{Context, Result};

    fn rem(writer: &MessageWriter) -> Result<&[u8]> {
        writer.remainder().context("empty remainder")
    }

    #[test]
    fn test_single() -> Result<()> {
        let mut writer = MessageWriter::new();
        assert_eq!(writer.remainder(), None);

        let msg = Message::new(NonEmptyInlineString::new("FOO")?)?;
        writer.push(&msg);
        assert_eq!(rem(&mut writer)?, &msg.encode());
        writer.written(non_zero_usize(100)?)?;
        assert_eq!(rem(&mut writer)?, &msg.encode()[100..]);
        writer.written(non_zero_usize(Message::BYTESIZE - 100)?)?;
        assert_eq!(writer.remainder(), None);

        Ok(())
    }

    #[test]
    fn test_fixed_size_queue_like_with_tail_replacement() -> Result<()> {
        let mut writer = MessageWriter::new();

        let msg1 = Message::new(NonEmptyInlineString::new("msg1")?)?;
        writer.push(&msg1);
        let msg2 = Message::new(NonEmptyInlineString::new("msg2")?)?;
        writer.push(&msg2);
        let msg3 = Message::new(NonEmptyInlineString::new("msg3")?)?;
        writer.push(&msg3);

        assert_eq!(rem(&mut writer)?, &msg1.encode());
        writer.written(non_zero_usize(Message::BYTESIZE)?)?;
        assert_eq!(rem(&mut writer)?, &msg3.encode());

        Ok(())
    }
}
