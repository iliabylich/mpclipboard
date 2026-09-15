use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{ID, REvents, UpgradeResponseWriter, prelude::*};
use rustix::event::{PollFd, PollFlags};
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
    ) -> Completion<(ID, OwnedFd), anyhow::Error, Self> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => return Failed(err.context(format!("[{self}] polling returned an error"))),
        };

        if revents.readable {
            unreachable!("[{self}] is readable but noone asked for it");
        }

        if revents.writable {
            log::trace!("[{self}] is writable");

            return match self.write(now) {
                Done(()) => Done((self.id, self.fd)),
                Failed(err) => Failed(err.context(format!("[{self}] write() failed for"))),
                Pending(()) => Pending(self),
            };
        }

        Pending(self)
    }

    fn write(&mut self, now: u64) -> Completion<(), anyhow::Error, ()> {
        self.last_activity_at = now;

        let buf = match self.writer.remainder() {
            Ok(buf) => buf,
            Err(err) => return Failed(err),
        };

        let len = match mpclipboard_shared::io::write(&self.fd, buf) {
            Done(len) => len,
            Failed(err) => return Failed(err),
            Pending(()) => return Pending(()),
        };

        self.writer
            .written(len)
            .map_err(|err| err.context(format!("{self} failed to call UpgradeResponseWriter")))
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
