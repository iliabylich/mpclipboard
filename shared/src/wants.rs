#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Read,
    Write,
    ReadWrite,
}
