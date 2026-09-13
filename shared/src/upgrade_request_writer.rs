use crate::{UpgradeRequest, writer::Writer};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter {
    inner: Writer<{ UpgradeRequest::BYTESIZE }>,
}

impl UpgradeRequestWriter {
    pub fn new(request: &UpgradeRequest) -> Self {
        Self {
            inner: Writer::new(request.encode()),
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
