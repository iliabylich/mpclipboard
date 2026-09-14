use crate::{prelude::*, upgrade_response::UpgradeResponse};
use anyhow::anyhow;
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

    pub fn written(&mut self, len: NonZeroUsize) -> Completion<(), anyhow::Error, ()> {
        self.pos = self
            .pos
            .checked_add(len.get())
            .unwrap_or_else(|| unreachable!("length overflow"));

        match self.pos.cmp(&UpgradeResponse::BYTES.len()) {
            core::cmp::Ordering::Less => Pending(()),
            core::cmp::Ordering::Equal => Done(()),
            core::cmp::Ordering::Greater => Failed(anyhow!("buffer overflow")),
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
    use crate::upgrade_response::UpgradeResponse;
    use core::num::NonZeroUsize;

    #[test]
    fn test_write() {
        let mut w = UpgradeResponseWriter::new();
        assert_eq!(w.remainder(), UpgradeResponse::BYTES);

        w.written(NonZeroUsize::new(50).unwrap()).unwrap_pending();
        assert_eq!(w.remainder(), &UpgradeResponse::BYTES[50..]);

        w.written(NonZeroUsize::new(UpgradeResponse::BYTES.len() - 50).unwrap())
            .unwrap();
        assert_eq!(w.remainder(), b"");

        let err = w.written(NonZeroUsize::new(1).unwrap()).unwrap_err();
        assert_eq!(err.to_string(), "buffer overflow");
    }
}
