use crate::Wants;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};

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

    fn transition(&mut self, next: Option<(BorrowedFd<'_>, Epoch, Wants)>) -> Diff {
        match (*self, next) {
            (Self::None, None) => Diff::Empty,
            (Self::None, Some((fd, epoch, wants))) => {
                *self = Self::Some(fd.as_raw_fd(), epoch, wants);
                Diff::Add {
                    fd: fd.as_raw_fd(),
                    wants,
                }
            }
            (Self::Some(prevfd, _, _), None) => {
                *self = Self::None;
                Diff::Delete { fd: prevfd }
            }
            (Self::Some(prevfd, prevepoch, prevwants), Some((fd, nextepoch, wants))) => {
                if nextepoch == prevepoch {
                    assert_eq!(fd.as_raw_fd(), prevfd, "fd changed without an epoch bump");

                    if wants == prevwants {
                        Diff::Empty
                    } else {
                        *self = Self::Some(prevfd, nextepoch, wants);
                        Diff::Modify { fd: prevfd, wants }
                    }
                } else {
                    *self = Self::Some(fd.as_raw_fd(), nextepoch, wants);
                    Diff::Replace {
                        prevfd,
                        newfd: fd.as_raw_fd(),
                        wants,
                    }
                }
            }
        }
    }
}

#[must_use]
#[derive(Debug)]
enum Diff {
    Add {
        fd: RawFd,
        wants: Wants,
    },
    Delete {
        fd: RawFd,
    },
    Modify {
        fd: RawFd,
        wants: Wants,
    },
    Replace {
        prevfd: RawFd,
        newfd: RawFd,
        wants: Wants,
    },
    Empty,
}
