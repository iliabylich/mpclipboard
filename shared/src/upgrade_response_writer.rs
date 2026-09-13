use crate::{upgrade_response::UpgradeResponse, writer::Writer};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseWriter {
    inner: Writer<{ UpgradeResponse::BYTESIZE }>,
}

impl UpgradeResponseWriter {
    pub const fn new() -> Self {
        let mut buf = [0; UpgradeResponse::BYTESIZE];
        buf.copy_from_slice(UpgradeResponse::BYTES);

        Self {
            inner: Writer::new(buf),
        }
    }

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.inner.remainder()
    }

    pub fn written(&mut self, len: NonZeroUsize) -> bool {
        self.inner.written(len)
    }
}

impl Default for UpgradeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}
