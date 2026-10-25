use crate::marble::MarbleRender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActualTick {
    NoTick,
    TickAt(u8),
    Multiple,
}

impl ActualTick {
    pub(crate) const fn record(&mut self, now: u8) {
        *self = match self {
            Self::NoTick => Self::TickAt(now),
            Self::TickAt(_) | Self::Multiple => Self::Multiple,
        };
    }
}

impl MarbleRender for ActualTick {
    type Output = char;

    fn render(&self) -> char {
        match self {
            Self::NoTick => '-',
            Self::TickAt(now) => (now + b'0') as char,
            Self::Multiple => '+',
        }
    }
}
