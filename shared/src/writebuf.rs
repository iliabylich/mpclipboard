use core::{cmp::Ordering, num::NonZeroUsize};

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

    pub(crate) fn written(&mut self, n: NonZeroUsize) -> bool {
        self.pos = self
            .pos
            .checked_add(n.get())
            .unwrap_or_else(|| unreachable!("overflow: n is too large"));

        match (self.pos).cmp(&N) {
            Ordering::Less => false,
            Ordering::Equal => {
                self.pos = 0;
                self.buf = [0; _];
                true
            }
            Ordering::Greater => unreachable!("buffer overflow"),
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
