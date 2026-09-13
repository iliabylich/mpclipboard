use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use core::num::NonZeroUsize;
use mpclipboard_shared::{
    Completion::{self, *},
    ID, REvents, UpgradeResponseWriter, error, trace,
};
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSink {
    fd: OwnedFd,
    id: ID,
    writer: UpgradeResponseWriter,
    last_activity_at: u64,
}

impl PreSink {
    pub(crate) const fn new(fd: OwnedFd, id: ID, now: u64) -> Self {
        Self {
            fd,
            id,
            writer: UpgradeResponseWriter::new(),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
        now: u64,
    ) -> Completion<(ID, OwnedFd), PreSink> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return Failed;
            }
        };

        if revents.readable {
            unreachable!("{self} is readable but noone asked for it");
        }

        if revents.writable {
            trace!("{self} is writable");

            let len =
                match rustix::io::write(&self.fd, self.writer.remainder()).map(NonZeroUsize::new) {
                    Ok(Some(len)) => len,
                    Err(Errno::AGAIN) => {
                        self.last_activity_at = now;
                        return Pending(self);
                    }
                    Ok(None) => {
                        error!("write() returned zero for {self}");
                        return Failed;
                    }
                    Err(errno) => {
                        error!("{self} failed to write(): {errno:?}");
                        return Failed;
                    }
                };

            match self.writer.written(len) {
                Done(()) => {
                    return Done((self.id, self.fd));
                }
                Pending(()) => {
                    self.last_activity_at = now;
                    return Pending(self);
                }
                Failed => {
                    error!("{self} failed to call UpgradeResponseWriter");
                    return Failed;
                }
            }
        }

        Pending(self)
    }
}

impl CanBeReaped for PreSink {
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl core::fmt::Display for PreSink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSink(fd={}, last_activity_at={})",
            self.fd.as_raw_fd(),
            self.last_activity_at
        )
    }
}

impl AsFd for PreSink {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for PreSink {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for PreSink {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::OUT)
    }
}
