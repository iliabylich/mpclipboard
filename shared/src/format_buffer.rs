pub struct FormatBuffer<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> FormatBuffer<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl<const N: usize> Default for FormatBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> core::fmt::Write for FormatBuffer<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let spare = &mut self.buf[self.len..];
        let len = s.len().min(spare.len());
        spare[..len].copy_from_slice(&s.as_bytes()[..len]);
        self.len += len;
        if len < s.len() {
            return Err(core::fmt::Error);
        }
        Ok(())
    }
}
