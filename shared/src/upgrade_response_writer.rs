use crate::upgrade_response::UpgradeResponse;
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

    pub fn written(&mut self, len: NonZeroUsize) -> UpgradeResponseWriterResult {
        self.pos = self
            .pos
            .checked_add(len.get())
            .unwrap_or_else(|| unreachable!("length overflow"));

        match self.pos.cmp(&UpgradeResponse::BYTES.len()) {
            core::cmp::Ordering::Less => UpgradeResponseWriterResult::Pending,
            core::cmp::Ordering::Equal => UpgradeResponseWriterResult::Done,
            core::cmp::Ordering::Greater => UpgradeResponseWriterResult::Error,
        }
    }
}

impl Default for UpgradeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpgradeResponseWriterResult {
    Done,
    Pending,
    Error,
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::UpgradeResponseWriter;
    use crate::{
        upgrade_response::UpgradeResponse, upgrade_response_writer::UpgradeResponseWriterResult,
    };

    #[test]
    fn test_write() {
        let mut w = UpgradeResponseWriter::new();
        assert_eq!(w.remainder(), UpgradeResponse::BYTES);

        assert_eq!(
            w.written(NonZeroUsize::new(50).unwrap()),
            UpgradeResponseWriterResult::Pending
        );
        assert_eq!(w.remainder(), &UpgradeResponse::BYTES[50..]);

        assert_eq!(
            w.written(NonZeroUsize::new(UpgradeResponse::BYTES.len() - 50).unwrap()),
            UpgradeResponseWriterResult::Done
        );
        assert_eq!(w.remainder(), b"");

        assert_eq!(
            w.written(NonZeroUsize::new(1).unwrap()),
            UpgradeResponseWriterResult::Error
        );
    }
}
