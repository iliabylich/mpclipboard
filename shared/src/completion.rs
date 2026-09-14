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
    pub fn unwrap(self) -> S
    where
        P: Debug,
        E: Debug,
    {
        match self {
            Self::Done(v) => v,
            Self::Failed(err) => panic!("expected Ok, got Err({err:?})"),
            Self::Pending(p) => panic!("expected Ok, got Pending({p:?})"),
        }
    }

    #[cfg(test)]
    pub fn unwrap_pending(self) -> P
    where
        S: Debug,
        E: Debug,
    {
        match self {
            Self::Done(v) => panic!("expected Pending, got Done({v:?}"),
            Self::Failed(err) => panic!("expected Pending, got Err({err:?})"),
            Self::Pending(p) => p,
        }
    }

    #[cfg(test)]
    pub fn unwrap_err(self) -> E
    where
        S: Debug,
        P: Debug,
    {
        match self {
            Self::Done(v) => panic!("expected Err, got Done({v:?}"),
            Self::Failed(err) => err,
            Self::Pending(p) => panic!("expected Err, got Pending({p:?})"),
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
