#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Read,
    Write,
    ReadWrite,
}

impl Wants {
    pub fn merge(self, other: Self) -> Self {
        let read = self.wants_read() || other.wants_read();
        let write = self.wants_write() || other.wants_write();

        match (read, write) {
            (true, true) => Self::ReadWrite,
            (true, false) => Self::Read,
            (false, true) => Self::Write,
            (false, false) => unreachable!("Wants always wants at least one event"),
        }
    }

    pub fn merge_opt(self, other: Option<Self>) -> Self {
        let mut out = self;
        if let Some(other) = other {
            out = out.merge(other);
        }
        out
    }

    pub(crate) const fn wants_read(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub(crate) const fn wants_write(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}
