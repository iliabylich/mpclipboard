use crate::{prelude::*, upgrade_response::UpgradeResponse};
use core::{cmp::Ordering, num::NonZeroUsize};

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
            .unwrap_or_else(|| unreachable!("pos never exceeds UpgradeResponse::BYTES.len()"))
    }

    pub fn written(&mut self, len: NonZeroUsize) -> Completion<(), UpgradeResponseWriterError, ()> {
        match self
            .pos
            .checked_add(len.get())
            .map(|nextpos| (nextpos, nextpos.cmp(&UpgradeResponse::BYTES.len())))
        {
            Some((nextpos, Ordering::Less)) => {
                self.pos = nextpos;
                Pending(())
            }
            Some((nextpos, Ordering::Equal)) => {
                self.pos = nextpos;
                Done(())
            }
            None | Some((_, Ordering::Greater)) => Failed(UpgradeResponseWriterError {
                written: len.get(),
                remaining: self.remainder().len(),
            }),
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

        assert_eq!(w.written(non_zero_usize(50)), Pending(()));
        assert_eq!(Some(w.remainder()), UpgradeResponse::BYTES.get(50..));

        assert_eq!(
            w.written(non_zero_usize(UpgradeResponse::BYTES.len() - 50)),
            Done(())
        );
        assert_eq!(w.remainder(), b"");

        assert_eq!(
            w.written(non_zero_usize(1)),
            Failed(UpgradeResponseWriterError {
                written: 1,
                remaining: 0,
            })
        );
    }
}
