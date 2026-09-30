#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch(u8);

impl Epoch {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Self::new()
    }
}
