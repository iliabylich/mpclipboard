use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{
    Completion::{self, *},
    REvents, UpgradeRequest, UpgradeRequestReader, error, trace,
};
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use std::{
    num::NonZeroUsize,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
};

pub struct PreSource {
    fd: OwnedFd,
    reader: UpgradeRequestReader,
    last_activity_at: u64,
}

impl PreSource {
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: UpgradeRequestReader::new(),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
        now: u64,
    ) -> Completion<(UpgradeRequest, OwnedFd), PreSource> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return Failed;
            }
        };

        if revents.writable {
            unreachable!("{self} is writable but noone asked for it");
        }

        if revents.readable {
            trace!("{self} is readable");

            let mut buf = [0; UpgradeRequestReader::BUFFER_SIZE];
            let len = match rustix::io::read(&self.fd, &mut buf).map(NonZeroUsize::new) {
                Ok(Some(len)) => len,
                Err(Errno::AGAIN) => return Pending(self),
                Ok(None) => {
                    error!("{self} reached EOF");
                    return Failed;
                }
                Err(errno) => {
                    error!("{self} failed to read(): {errno:?}");
                    return Failed;
                }
            };

            match self.reader.received(buf, len) {
                Done(req) => {
                    return Done((req, self.fd));
                }
                Pending(()) => {
                    self.last_activity_at = now;
                    return Pending(self);
                }
                Failed => {
                    error!("{self} got an error from UpgradeRequestReader");
                    return Failed;
                }
            }
        }

        Pending(self)
    }
}

impl CanBeReaped for PreSource {
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl core::fmt::Display for PreSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSource(fd={}, last_activity_at={})",
            self.fd.as_raw_fd(),
            self.last_activity_at
        )
    }
}

impl AsFd for PreSource {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for PreSource {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for PreSource {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}
