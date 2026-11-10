use core::{
    future::{Future, poll_fn},
    pin::pin,
    task::Poll,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Either<L, R> {
    Left(L),
    Right(R),
}

pub(crate) async fn select<L: Future, R: Future>(
    left: L,
    right: R,
) -> Either<L::Output, R::Output> {
    let mut left = pin!(left);
    let mut right = pin!(right);
    poll_fn(|cx| {
        if let Poll::Ready(output) = left.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(output));
        }
        if let Poll::Ready(output) = right.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(output));
        }
        Poll::Pending
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{Either, select};
    use core::{
        future::{Future, pending, ready},
        pin::pin,
        task::{Context, Poll, Waker},
    };

    fn poll_once<F: Future>(future: F) -> Poll<F::Output> {
        pin!(future).poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn test_left() {
        assert_eq!(
            poll_once(select(ready(1), pending::<()>())),
            Poll::Ready(Either::Left(1))
        );
    }

    #[test]
    fn test_right() {
        assert_eq!(
            poll_once(select(pending::<()>(), ready(2))),
            Poll::Ready(Either::Right(2))
        );
    }

    #[test]
    fn test_left_wins_when_both_ready() {
        assert_eq!(
            poll_once(select(ready(1), ready(2))),
            Poll::Ready(Either::Left(1))
        );
    }

    #[test]
    fn test_pending() {
        assert_eq!(
            poll_once(select(pending::<()>(), pending::<()>())),
            Poll::Pending
        );
    }
}
