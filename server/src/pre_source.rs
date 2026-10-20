use crate::reaper::CanBeReaped;
use anyhow::{Context, Result, ensure};
use bytes::BytesMut;
use mpclipboard_shared::{
    EventLoopFdResult, Now, Past, RegisteredFd, Request, RequestReader, prelude::*,
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

pub struct PreSource {
    fd: RegisteredFd,
    reader: RequestReader,
    created_at: Past,
}

impl PreSource {
    const READ_SIZE: usize = 8192;

    pub(crate) fn new(fd: RegisteredFd, now: Now) -> Self {
        Self {
            fd,
            reader: RequestReader::new(),
            created_at: now.as_past(),
        }
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: EventLoopFdResult,
    ) -> Result<Completion<(Request, BytesMut, RegisteredFd), Self>> {
        ensure!(!revents.has_error, "[{self}] polling returned an error");

        if revents.readable {
            log::trace!("[{self}] is readable");

            let Done((req, leftover)) = self
                .read()
                .with_context(|| format!("[{self}] read() failed"))?
            else {
                return Ok(Pending(self));
            };
            return Ok(Done((req, leftover, self.fd)));
        }

        Ok(Pending(self))
    }

    fn read(&mut self) -> Result<Completion<(Request, BytesMut), ()>> {
        let Done(buf) = mpclipboard_shared::io::read::<{ Self::READ_SIZE }>(&self.fd)? else {
            return Ok(Pending(()));
        };

        let received = self.reader.received(&buf)?;
        Ok(received)
    }
}

impl CanBeReaped for PreSource {
    fn created_at(&self) -> Past {
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
