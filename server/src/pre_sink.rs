use crate::reaper::CanBeReaped;
use anyhow::{Context, Result, ensure};
use bytes::BytesMut;
use mpclipboard_shared::{EventLoopAwareFd, EventLoopFdResult, ID, Now, Past, Wants, prelude::*};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

pub(crate) enum Verdict {
    Accepted { id: ID, leftover: BytesMut },
    Unauthorized { id: ID },
}

impl Verdict {
    const fn id(&self) -> &ID {
        match self {
            Self::Accepted { id, .. } | Self::Unauthorized { id } => id,
        }
    }

    const fn response(&self) -> &'static [u8] {
        match self {
            Self::Accepted { .. } => b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n",
            Self::Unauthorized { .. } => b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n",
        }
    }
}

pub struct PreSink {
    fd: EventLoopAwareFd,
    verdict: Verdict,
    remainder: &'static [u8],
    created_at: Past,
}

impl PreSink {
    pub(crate) fn new(fd: EventLoopAwareFd, verdict: Verdict, now: Now) -> Result<Self> {
        fd.modify(Wants::Write)
            .with_context(|| format!("[{}] failed to wait for writability", verdict.id()))?;

        Ok(Self {
            fd,
            remainder: verdict.response(),
            verdict,
            created_at: now.as_past(),
        })
    }

    pub(crate) const fn id(&self) -> &ID {
        self.verdict.id()
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: EventLoopFdResult,
    ) -> Result<Completion<(Verdict, EventLoopAwareFd), Self>> {
        ensure!(!revents.has_error, "[{self}] polling returned an error");

        if revents.writable {
            log::trace!("[{self}] is writable");

            let Done(()) = self
                .write()
                .with_context(|| format!("[{self}] write() failed"))?
            else {
                return Ok(Pending(self));
            };
            return Ok(Done((self.verdict, self.fd)));
        }

        Ok(Pending(self))
    }

    fn write(&mut self) -> Result<Completion<(), ()>> {
        let Done(len) = mpclipboard_shared::io::write(&self.fd, self.remainder)? else {
            return Ok(Pending(()));
        };

        self.remainder = self.remainder.get(len.get()..).with_context(|| {
            format!(
                "[{self}] write() reported {len} bytes, but only {} bytes remained",
                self.remainder.len()
            )
        })?;

        if self.remainder.is_empty() {
            Ok(Done(()))
        } else {
            Ok(Pending(()))
        }
    }
}

impl CanBeReaped for PreSink {
    fn created_at(&self) -> Past {
        self.created_at
    }
}

impl core::fmt::Display for PreSink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSink(fd={}, created_at={})",
            self.fd.as_fd().as_raw_fd(),
            self.created_at
        )
    }
}

impl AsFd for PreSink {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
