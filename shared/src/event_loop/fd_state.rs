use crate::{Wants, event_loop::Epoch};
use rustix::fd::{AsRawFd, BorrowedFd, RawFd};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub enum FdState {
    None,
    Some(RawFd, Epoch, Wants),
}

impl FdState {
    pub const fn new() -> Self {
        Self::None
    }

    /// # Panics
    ///
    /// If `next` has the same epoch as the current state but a different fd.
    pub fn transition<'fd>(&mut self, next: Option<(BorrowedFd<'fd>, Epoch, Wants)>) -> Diff<'fd> {
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

impl Default for FdState {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
#[derive(Debug)]
pub enum Diff<'fd> {
    Add { fd: BorrowedFd<'fd>, wants: Wants },
    Modify { fd: BorrowedFd<'fd>, wants: Wants },
    Empty,
}
