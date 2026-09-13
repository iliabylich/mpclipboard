use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{
    REvents, UpgradeRequest, UpgradeRequestReader, UpgradeRequestReaderResult, error, trace,
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

#[expect(clippy::large_enum_variant)]
pub enum PreSourceResult {
    Died,
    Pending(PreSource),
    Done((UpgradeRequest, OwnedFd)),
}

impl PreSource {
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: UpgradeRequestReader::new(),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags, now: u64) -> PreSourceResult {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return PreSourceResult::Died;
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
                Err(Errno::AGAIN) => return PreSourceResult::Pending(self),
                Ok(None) => {
                    error!("{self} reached EOF");
                    return PreSourceResult::Died;
                }
                Err(errno) => {
                    error!("{self} failed to read(): {errno:?}");
                    return PreSourceResult::Died;
                }
            };

            match self.reader.received(buf, len) {
                UpgradeRequestReaderResult::Done {
                    req,
                    leftover: _leftover,
                    leftover_len,
                } => {
                    if leftover_len != 0 {
                        error!("{self} provided additional bytes after upgrade request");
                        return PreSourceResult::Died;
                    }
                    return PreSourceResult::Done((req, self.fd));
                }
                UpgradeRequestReaderResult::Pending => {
                    self.last_activity_at = now;
                    return PreSourceResult::Pending(self);
                }
                UpgradeRequestReaderResult::Error => {
                    error!("{self} got an error from UpgradeRequestReader");
                    return PreSourceResult::Died;
                }
            }
        }

        PreSourceResult::Pending(self)
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
