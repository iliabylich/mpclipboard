use crate::Wants;
use std::os::fd::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod epoll;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use epoll::{EpollError, EventLoop};

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
    Some(RawFd, Wants),
}

impl FdState {
    const fn new() -> Self {
        Self::None
    }

    fn transition(&mut self, next: Option<(RawFd, Wants)>) -> Diff {
        match (*self, next) {
            (Self::None, None) => Diff::Empty,
            (Self::None, Some((fd, wants))) => {
                *self = Self::Some(fd, wants);
                Diff::Add { fd, wants }
            }
            (Self::Some(prevfd, _), None) => {
                *self = Self::None;
                Diff::Delete { fd: prevfd }
            }
            (Self::Some(prevfd, prevwants), Some((fd, wants))) => {
                if fd != prevfd {
                    *self = Self::Some(fd, wants);
                    Diff::Replace {
                        prevfd,
                        newfd: fd,
                        wants,
                    }
                } else if wants != prevwants {
                    *self = Self::Some(fd, wants);
                    Diff::Modify { fd, wants }
                } else {
                    Diff::Empty
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
