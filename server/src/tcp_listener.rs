use crate::as_poll_fd::AsPollFd;
use anyhow::{Context, Result};
use core::net::SocketAddrV4;
use mpclipboard_shared::REvents;
use rustix::{
    event::{PollFd, PollFlags},
    net::{AddressFamily, SocketType},
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct TcpListener {
    fd: OwnedFd,
}

impl TcpListener {
    pub(crate) fn new(addr: SocketAddrV4) -> Result<Self> {
        let fd = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)
            .context("failed to socket()")?;
        rustix::net::sockopt::set_socket_reuseaddr(&fd, true)?;
        rustix::io::ioctl_fionbio(&fd, true)?;
        rustix::net::bind(&fd, &addr).context("failed to bind()")?;
        rustix::net::listen(&fd, 256).context("failed to listen()")?;
        log::info!("Listening on http://{addr}");

        Ok(Self { fd })
    }

    pub(crate) fn accept(&self, revents: PollFlags) -> Result<Option<OwnedFd>> {
        let revents = REvents::new(revents)?;

        if !revents.readable {
            return Ok(None);
        }

        let fd = rustix::net::accept(self)?;
        rustix::io::ioctl_fionbio(&fd, true)?;
        Ok(Some(fd))
    }
}

impl AsFd for TcpListener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for TcpListener {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}
