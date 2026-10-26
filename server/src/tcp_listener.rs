use crate::fd_set::key;
use anyhow::{Context, Result, ensure};
use core::net::SocketAddrV4;
use mpclipboard_shared::{EventLoopAwareFd, EventLoopFdResult, EventLoopWithTimer, Wants};
use rustix::net::{AddressFamily, SocketType};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

pub struct TcpListener {
    fd: EventLoopAwareFd,
}

impl TcpListener {
    pub(crate) fn new(addr: SocketAddrV4, event_loop: &EventLoopWithTimer) -> Result<Self> {
        let fd = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)
            .context("failed to socket()")?;
        rustix::net::sockopt::set_socket_reuseaddr(&fd, true)?;
        rustix::io::ioctl_fionbio(&fd, true)?;
        rustix::net::bind(&fd, &addr).context("failed to bind()")?;
        rustix::net::listen(&fd, 256).context("failed to listen()")?;
        log::info!("Listening on http://{addr}");

        let key = key(&fd);
        let fd = event_loop
            .register(fd, key, Wants::Read)
            .context("failed to register listener")?;

        Ok(Self { fd })
    }

    pub(crate) fn accept(&self, revents: EventLoopFdResult) -> Result<Option<OwnedFd>> {
        ensure!(!revents.has_error, "polling returned an error");

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
