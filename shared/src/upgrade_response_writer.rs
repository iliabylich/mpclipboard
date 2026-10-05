use crate::{prelude::*, upgrade_response::UpgradeResponse};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseWriter {
    remainder: &'static [u8],
}

impl UpgradeResponseWriter {
    pub const fn new() -> Self {
        Self {
            remainder: UpgradeResponse::BYTES,
        }
    }

    #[must_use]
    pub const fn remainder(&self) -> &[u8] {
        self.remainder
    }

    pub fn written(
        &mut self,
        len: NonZeroUsize,
    ) -> Result<Completion<(), ()>, UpgradeResponseWriterError> {
        let (_, rest) = self.remainder.split_at_checked(len.get()).ok_or_else(|| {
            UpgradeResponseWriterError {
                written: len.get(),
                remaining: self.remainder.len(),
            }
        })?;
        self.remainder = rest;

        if self.remainder.is_empty() {
            Ok(Done(()))
        } else {
            Ok(Pending(()))
        }
    }
}

impl Default for UpgradeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("written() reported {written} bytes, but only {remaining} bytes remained")]
pub struct UpgradeResponseWriterError {
    pub written: usize,
    pub remaining: usize,
}

#[cfg(test)]
mod tests {
    use super::{UpgradeResponseWriter, UpgradeResponseWriterError};
    use crate::{prelude::*, test_helpers::non_zero_usize, upgrade_response::UpgradeResponse};

    #[test]
    fn test_write() {
        let mut w = UpgradeResponseWriter::new();
        assert_eq!(w.remainder(), UpgradeResponse::BYTES);

        assert_eq!(w.written(non_zero_usize(50)), Ok(Pending(())));
        assert_eq!(Some(w.remainder()), UpgradeResponse::BYTES.get(50..));

        assert_eq!(
            w.written(non_zero_usize(UpgradeResponse::BYTES.len() - 50)),
            Ok(Done(()))
        );
        assert_eq!(w.remainder(), b"");

        assert_eq!(
            w.written(non_zero_usize(1)),
            Err(UpgradeResponseWriterError {
                written: 1,
                remaining: 0,
            })
        );
    }
}
