use crate::{
    Context, Output,
    state::{Connected, Established},
};
use anyhow::Result;
use std::os::fd::{AsRawFd, OwnedFd};

pub(crate) struct Establishing {
    fd: OwnedFd,
    started_at: u64,
}

impl Establishing {
    pub(crate) const fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            started_at: now,
        }
    }

    pub(crate) fn finish_connecting(
        self,
        context: &Context,
    ) -> Result<(Connected, Option<Output>)> {
        log::trace!("finish connecting");
        let now = context.timer.now();

        match rustix::net::sockopt::socket_error(&self.fd) {
            Ok(Ok(())) => {
                context.event_loop.modify(self.fd.as_raw_fd(), true, true)?;

                Ok((Connected::Established(Established::new(self.fd, now)), None))
            }
            Ok(Err(err)) | Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub(crate) const fn should_disconnect_at(&self) -> u64 {
        self.started_at.wrapping_add(5)
    }
}

impl AsRawFd for Establishing {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
