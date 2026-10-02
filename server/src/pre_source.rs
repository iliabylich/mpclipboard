use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use anyhow::anyhow;
use mpclipboard_shared::{REvents, UpgradeRequest, UpgradeRequestReader, prelude::*};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource {
    fd: OwnedFd,
    reader: UpgradeRequestReader,
    created_at: u64,
}

impl PreSource {
    pub(crate) const fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: UpgradeRequestReader::new(),
            created_at: now,
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
    ) -> Completion<(UpgradeRequest, OwnedFd), anyhow::Error, Self> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                return Failed(anyhow!(err).context(format!("[{self}] polling returned an error")));
            }
        };

        if revents.writable {
            unreachable!("[{self}] is writable but noone asked for it");
        }

        if revents.readable {
            log::trace!("[{self}] is readable");

            return match self.read() {
                Done(req) => Done((req, self.fd)),
                Failed(err) => Failed(err.context(format!("[{self}] read() failed for"))),
                Pending(()) => Pending(self),
            };
        }

        Pending(self)
    }

    fn read(&mut self) -> Completion<UpgradeRequest, anyhow::Error, ()> {
        let buf = match mpclipboard_shared::io::read(&self.fd) {
            Done(buf) => buf,
            Failed(err) => return Failed(err.into()),
            Pending(()) => return Pending(()),
        };

        self.reader.received(buf).map_err(|err| anyhow!(err))
    }
}

impl CanBeReaped for PreSource {
    fn created_at(&self) -> u64 {
        self.created_at
    }
}

impl core::fmt::Display for PreSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSource(fd={}, created_at={})",
            self.fd.as_raw_fd(),
            self.created_at
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
