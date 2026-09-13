use crate::{Completion, upgrade_response::UpgradeResponse};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseWriter {
    pos: usize,
}

impl UpgradeResponseWriter {
    pub const fn new() -> Self {
        Self { pos: 0 }
    }

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        UpgradeResponse::BYTES
            .get(self.pos..)
            .unwrap_or_else(|| unreachable!("malformed state"))
    }

    pub fn written(&mut self, len: NonZeroUsize) -> Completion<(), ()> {
        self.pos = self
            .pos
            .checked_add(len.get())
            .unwrap_or_else(|| unreachable!("length overflow"));

        match self.pos.cmp(&UpgradeResponse::BYTES.len()) {
            core::cmp::Ordering::Less => Completion::Pending(()),
            core::cmp::Ordering::Equal => Completion::Done(()),
            core::cmp::Ordering::Greater => Completion::Failed,
        }
    }
}

impl Default for UpgradeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeResponseWriter;
    use crate::{Completion, upgrade_response::UpgradeResponse};
    use core::num::NonZeroUsize;

    #[test]
    fn test_write() {
        let mut w = UpgradeResponseWriter::new();
        assert_eq!(w.remainder(), UpgradeResponse::BYTES);

        assert_eq!(
            w.written(NonZeroUsize::new(50).unwrap()),
            Completion::Pending(())
        );
        assert_eq!(w.remainder(), &UpgradeResponse::BYTES[50..]);

        assert_eq!(
            w.written(NonZeroUsize::new(UpgradeResponse::BYTES.len() - 50).unwrap()),
            Completion::Done(())
        );
        assert_eq!(w.remainder(), b"");

        assert_eq!(w.written(NonZeroUsize::new(1).unwrap()), Completion::Failed);
    }
}
