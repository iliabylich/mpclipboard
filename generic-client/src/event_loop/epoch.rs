#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Epoch(u8);

impl Epoch {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) const fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
