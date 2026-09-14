#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Completion<S, P> {
    Done(S),
    Failed,
    Pending(P),
}

impl<S, P> Completion<S, P> {
    #[cfg(test)]
    pub fn unwrap(self) -> S
    where
        P: core::fmt::Debug,
    {
        match self {
            Self::Done(value) => value,
            Self::Failed => panic!("expected Ok, got Err"),
            Self::Pending(pending) => panic!("expected Ok, got Pending({pending:?})"),
        }
    }

    pub fn and_then<T, F>(self, f: F) -> Completion<T, P>
    where
        F: FnOnce(S) -> Completion<T, P>,
    {
        match self {
            Self::Done(v) => f(v),
            Self::Failed => Completion::Failed,
            Self::Pending(pending) => Completion::Pending(pending),
        }
    }

    pub fn map<T, F>(self, f: F) -> Completion<T, P>
    where
        F: FnOnce(S) -> T,
    {
        match self {
            Self::Done(v) => Completion::Done(f(v)),
            Self::Failed => Completion::Failed,
            Self::Pending(pending) => Completion::Pending(pending),
        }
    }

    pub fn map_err<F>(self, f: F) -> Self
    where
        F: FnOnce(),
    {
        if matches!(self, Self::Failed) {
            f();
        }
        self
    }
}
