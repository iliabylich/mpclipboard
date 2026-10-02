use crate::{Wants, event_loop::Epoch};
use rustix::fd::{AsRawFd, BorrowedFd, RawFd};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub(crate) enum FdState {
    None,
    Some(RawFd, Epoch, Wants),
}

impl FdState {
    pub(crate) const fn new() -> Self {
        Self::None
    }

    pub(crate) fn transition<'fd>(
        &mut self,
        next: Option<(BorrowedFd<'fd>, Epoch, Wants)>,
    ) -> Diff<'fd> {
        match (*self, next) {
            (Self::None, None) => Diff::Empty,
            (Self::None, Some((fd, epoch, wants))) => {
                *self = Self::Some(fd.as_raw_fd(), epoch, wants);
                Diff::Add { fd, wants }
            }
            (Self::Some(..), None) => {
                *self = Self::None;
                Diff::Empty
            }
            (Self::Some(prevfd, prevepoch, prevwants), Some((fd, nextepoch, wants))) => {
                if nextepoch == prevepoch {
                    assert_eq!(fd.as_raw_fd(), prevfd, "fd changed without an epoch bump");

                    if wants == prevwants {
                        Diff::Empty
                    } else {
                        *self = Self::Some(prevfd, nextepoch, wants);
                        Diff::Modify { fd, wants }
                    }
                } else {
                    *self = Self::Some(fd.as_raw_fd(), nextepoch, wants);
                    Diff::Add { fd, wants }
                }
            }
        }
    }
}

#[must_use]
#[derive(Debug)]
pub(crate) enum Diff<'fd> {
    Add { fd: BorrowedFd<'fd>, wants: Wants },
    Modify { fd: BorrowedFd<'fd>, wants: Wants },
    Empty,
}
