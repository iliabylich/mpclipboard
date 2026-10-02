use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use anyhow::anyhow;
use mpclipboard_shared::{ID, REvents, UpgradeResponseWriter, prelude::*};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSink {
    fd: OwnedFd,
    id: ID,
    writer: UpgradeResponseWriter,
    created_at: u64,
}

impl PreSink {
    pub(crate) const fn new(fd: OwnedFd, id: ID, now: u64) -> Self {
        Self {
            fd,
            id,
            writer: UpgradeResponseWriter::new(),
            created_at: now,
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
    ) -> Completion<(ID, OwnedFd), anyhow::Error, Self> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                return Failed(anyhow!(err).context(format!("[{self}] polling returned an error")));
            }
        };

        if revents.readable {
            unreachable!("[{self}] is readable but noone asked for it");
        }

        if revents.writable {
            log::trace!("[{self}] is writable");

            return match self.write() {
                Done(()) => Done((self.id, self.fd)),
                Failed(err) => Failed(err.context(format!("[{self}] write() failed"))),
                Pending(()) => Pending(self),
            };
        }

        Pending(self)
    }

    fn write(&mut self) -> Completion<(), anyhow::Error, ()> {
        let buf = self.writer.remainder();

        let len = match mpclipboard_shared::io::write(&self.fd, buf) {
            Done(len) => len,
            Failed(err) => return Failed(err.into()),
            Pending(()) => return Pending(()),
        };

        self.writer.written(len).map_err(|err| {
            anyhow!(err).context(format!("{self} failed to call UpgradeResponseWriter"))
        })
    }
}

impl CanBeReaped for PreSink {
    fn created_at(&self) -> u64 {
        self.created_at
    }
}

impl core::fmt::Display for PreSink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSink(fd={}, created_at={})",
            self.fd.as_raw_fd(),
            self.created_at
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
