use crate::marble::MarbleRender;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActualOccurrence(pub(crate) u8);

impl ActualOccurrence {
    pub(crate) fn add(&mut self, n: u8) {
        self.0 = self.0.saturating_add(n);
    }
}

impl MarbleRender for ActualOccurrence {
    type Output = char;

    fn render(&self) -> Self::Output {
        match self.0 {
            0 => '-',
            count @ 1..=9 => (count + b'0') as char,
            _ => '+',
        }
    }
}
