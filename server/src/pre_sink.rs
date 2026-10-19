use crate::reaper::CanBeReaped;
use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{
    EventLoopFdResult, ID, RegisteredFd, UpgradeResponseWriter, Wants, prelude::*,
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

pub struct PreSink {
    fd: RegisteredFd,
    id: ID,
    writer: UpgradeResponseWriter,
    created_at: u64,
}

impl PreSink {
    pub(crate) fn new(fd: RegisteredFd, id: ID, now: u64) -> Result<Self> {
        fd.modify(Wants::Write)
            .with_context(|| format!("[{id}] failed to wait for writability"))?;

        Ok(Self {
            fd,
            id,
            writer: UpgradeResponseWriter::new(),
            created_at: now,
        })
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: EventLoopFdResult,
    ) -> Result<Completion<(ID, RegisteredFd), Self>> {
        ensure!(!revents.has_error, "[{self}] polling returned an error");

        if revents.writable {
            log::trace!("[{self}] is writable");

            let Done(()) = self
                .write()
                .with_context(|| format!("[{self}] write() failed"))?
            else {
                return Ok(Pending(self));
            };
            return Ok(Done((self.id, self.fd)));
        }

        Ok(Pending(self))
    }

    fn write(&mut self) -> Result<Completion<(), ()>> {
        let buf = self.writer.remainder();

        let Done(len) = mpclipboard_shared::io::write(&self.fd, buf)? else {
            return Ok(Pending(()));
        };

        self.writer
            .written(len)
            .with_context(|| format!("[{self}] failed to call UpgradeResponseWriter"))
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
