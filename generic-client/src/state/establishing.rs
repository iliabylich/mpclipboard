use crate::{
    Context, Output,
    state::{Connected, Established},
};
use anyhow::Result;
use std::os::fd::{AsRawFd, OwnedFd};

pub(crate) struct Establishing {
    fd: OwnedFd,
}

impl Establishing {
    pub(crate) const fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub(crate) fn finish_connecting(
        self,
        context: &Context,
    ) -> Result<(Connected, Option<Output>)> {
        log::trace!("finish connecting");

        match rustix::net::sockopt::socket_error(&self.fd) {
            Ok(Ok(())) => {
                context.event_loop.modify(self.fd.as_raw_fd(), true, true)?;

                Ok((Connected::Established(Established::new(self.fd)), None))
            }
            Ok(Err(err)) | Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
}

impl AsRawFd for Establishing {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
