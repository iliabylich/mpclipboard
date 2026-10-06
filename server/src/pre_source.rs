use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use anyhow::{Context, Result};
use mpclipboard_shared::{REvents, UpgradeRequest, UpgradeRequestReader, prelude::*};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource {
    fd: OwnedFd,
    reader: UpgradeRequestReader,
    created_at: u64,
}

impl PreSource {
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: UpgradeRequestReader::new(),
            created_at: now,
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
    ) -> Result<Completion<(UpgradeRequest, OwnedFd), Self>> {
        let revents =
            REvents::new(revents).with_context(|| format!("[{self}] polling returned an error"))?;

        if revents.writable {
            unreachable!("[{self}] is writable but noone asked for it");
        }

        if revents.readable {
            log::trace!("[{self}] is readable");

            let Done(req) = self
                .read()
                .with_context(|| format!("[{self}] read() failed"))?
            else {
                return Ok(Pending(self));
            };
            return Ok(Done((req, self.fd)));
        }

        Ok(Pending(self))
    }

    fn read(&mut self) -> Result<Completion<UpgradeRequest, ()>> {
        let Done(buf) = mpclipboard_shared::io::read(&self.fd)? else {
            return Ok(Pending(()));
        };

        let received = self.reader.received(buf)?;
        Ok(received)
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
