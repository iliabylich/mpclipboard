use core::fmt::Debug;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Completion<S, E, P> {
    Done(S),
    Failed(E),
    Pending(P),
}

impl<S, E, P> Completion<S, E, P> {
    #[cfg(test)]
    pub fn expect_done(self, s: &str) -> S
    where
        P: Debug,
        E: Debug,
    {
        match self {
            Self::Done(v) => v,
            Self::Failed(err) => panic!("expected Ok, got Err({err:?}): {s}"),
            Self::Pending(p) => panic!("expected Ok, got Pending({p:?}): {s}"),
        }
    }

    #[cfg(test)]
    pub fn expect_pending(self, s: &str) -> P
    where
        S: Debug,
        E: Debug,
    {
        match self {
            Self::Done(v) => panic!("expected Pending, got Done({v:?}: {s}"),
            Self::Failed(err) => panic!("expected Pending, got Err({err:?}): {s}"),
            Self::Pending(p) => p,
        }
    }

    #[cfg(test)]
    pub fn expect_failed(self, s: &str) -> E
    where
        S: Debug,
        P: Debug,
    {
        match self {
            Self::Done(v) => panic!("expected Err, got Done({v:?}: {s}"),
            Self::Failed(err) => err,
            Self::Pending(p) => panic!("expected Err, got Pending({p:?}): {s}"),
        }
    }

    pub fn and_then<T, F>(self, f: F) -> Completion<T, E, P>
    where
        F: FnOnce(S) -> Completion<T, E, P>,
    {
        match self {
            Self::Done(v) => f(v),
            Self::Failed(err) => Completion::Failed(err),
            Self::Pending(p) => Completion::Pending(p),
        }
    }

    pub fn map<T, F>(self, f: F) -> Completion<T, E, P>
    where
        F: FnOnce(S) -> T,
    {
        match self {
            Self::Done(v) => Completion::Done(f(v)),
            Self::Failed(err) => Completion::Failed(err),
            Self::Pending(p) => Completion::Pending(p),
        }
    }

    pub fn map_err<F, E2>(self, f: F) -> Completion<S, E2, P>
    where
        F: FnOnce(E) -> E2,
    {
        match self {
            Self::Done(v) => Completion::Done(v),
            Self::Failed(err) => Completion::Failed(f(err)),
            Self::Pending(p) => Completion::Pending(p),
        }
    }
}
