#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Read,
    Write,
    ReadWrite,
}

impl Wants {
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::ReadWrite, _)
            | (_, Self::ReadWrite)
            | (Self::Read, Self::Write)
            | (Self::Write, Self::Read) => Self::ReadWrite,

            (Self::Read, Self::Read) => Self::Read,

            (Self::Write, Self::Write) => Self::Write,
        }
    }

    pub const fn merge_opt(self, other: Option<Self>) -> Self {
        let mut out = self;
        if let Some(other) = other {
            out = out.merge(other);
        }
        out
    }
}
