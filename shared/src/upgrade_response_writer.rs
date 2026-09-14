use crate::{prelude::*, upgrade_response::UpgradeResponse};
use anyhow::{Context, Result, anyhow};
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

    pub fn remainder(&self) -> Result<&[u8]> {
        UpgradeResponse::BYTES
            .get(self.pos..)
            .context("malformed state")
    }

    pub fn written(&mut self, len: NonZeroUsize) -> Completion<(), anyhow::Error, ()> {
        let Some(nextpos) = self.pos.checked_add(len.get()) else {
            return Failed(anyhow!("length overflow"));
        };
        self.pos = nextpos;

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
    use crate::{test_helpers::non_zero_usize, upgrade_response::UpgradeResponse};
    use anyhow::Result;

    #[test]
    fn test_write() -> Result<()> {
        let mut w = UpgradeResponseWriter::new();
        assert_eq!(w.remainder()?, UpgradeResponse::BYTES);

        w.written(non_zero_usize(50)?)
            .expect_pending("only 50 bytes have been written");
        assert_eq!(w.remainder()?, &UpgradeResponse::BYTES[50..]);

        w.written(non_zero_usize(UpgradeResponse::BYTES.len() - 50)?)
            .expect_done("full response have been written");
        assert_eq!(w.remainder()?, b"");

        let err = w
            .written(non_zero_usize(1)?)
            .expect_failed("trying to go pas the buffer end");
        assert_eq!(err.to_string(), "buffer overflow");

        Ok(())
    }
}
