use crate::Wants;
use rustix::{
    fd::{AsRawFd, BorrowedFd, RawFd},
    io::Errno,
};

mod epoch;
pub use epoch::Epoch;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod epoll;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use epoll::EventLoop;

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub use kqueue::EventLoop;

#[derive(Debug)]
pub struct EventLoopResult {
    pub time: Option<u64>,
    pub fd: Option<(bool, bool, bool)>,
}

#[must_use]
#[derive(Debug, Clone, Copy)]
enum FdState {
    None,
    Some(RawFd, Epoch, Wants),
}

impl FdState {
    const fn new() -> Self {
        Self::None
    }

    fn transition<'fd>(&mut self, next: Option<(BorrowedFd<'fd>, Epoch, Wants)>) -> Diff<'fd> {
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
enum Diff<'fd> {
    Add { fd: BorrowedFd<'fd>, wants: Wants },
    Modify { fd: BorrowedFd<'fd>, wants: Wants },
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventLoopError {
    #[error("failed to create event loop: {0:?}")]
    Create(Errno),
    #[error("failed to create timer: {0:?}")]
    CreateTimer(Errno),
    #[error("failed to register timer: {0:?}")]
    AddTimer(Errno),
    #[error("failed to read timer: {0:?}")]
    ReadTimer(Errno),
    #[error("failed to register connection fd: {0:?}")]
    Sync(Errno),
    #[error("failed to wait for events: {0:?}")]
    Wait(Errno),
}
