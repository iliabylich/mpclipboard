#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Completion<T, P> {
    Done(T),
    Pending(P),
}
