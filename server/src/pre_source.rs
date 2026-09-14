use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{REvents, UpgradeRequest, UpgradeRequestReader, error, prelude::*, trace};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource {
    fd: OwnedFd,
    reader: UpgradeRequestReader,
    last_activity_at: u64,
}

impl PreSource {
    pub(crate) const fn new(fd: OwnedFd, now: u64) -> Self {
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
    ) -> Completion<(UpgradeRequest, OwnedFd), Self> {
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

            return match self.read(now) {
                Done(req) => Done((req, self.fd)),
                Failed => Failed,
                Pending(()) => Pending(self),
            };
        }

        Pending(self)
    }

    fn read(&mut self, now: u64) -> Completion<UpgradeRequest, ()> {
        self.last_activity_at = now;

        let mut buf = [0; UpgradeRequestReader::BUFFER_SIZE];
        mpclipboard_shared::io::read(&self.fd, &mut buf)
            .map_err(|| error!("read() failed for {self}"))
            .and_then(|len| {
                self.reader
                    .received(buf, len)
                    .map_err(|| error!("{self} failed to call UpgradeRequestReader"))
            })
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
