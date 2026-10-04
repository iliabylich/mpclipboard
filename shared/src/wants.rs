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

pub trait OptionWantsExt {
    #[must_use]
    fn merge_opt(self, other: Self) -> Self;
}

impl OptionWantsExt for Option<Wants> {
    fn merge_opt(self, other: Self) -> Self {
        match (self, other) {
            (Some(lhs), rhs) => Some(lhs.merge_opt(rhs)),
            (None, rhs) => rhs,
        }
    }
}
