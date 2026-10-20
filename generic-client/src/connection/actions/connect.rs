use core::net::SocketAddr;
use rustix::{
    fd::OwnedFd,
    io::{Errno, FdFlags},
    net::{AddressFamily, SocketType},
};

pub(crate) fn connect(addr: SocketAddr) -> Result<OwnedFd, ConnectError> {
    let fd = rustix::net::socket(AddressFamily::INET, SocketType::DGRAM, None)
        .map_err(ConnectError::Socket)?;

    rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectError::SetCloexec)?;

    rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectError::SwitchToNonBlocking)?;

    rustix::net::connect(&fd, &addr).map_err(ConnectError::Connect)?;

    Ok(fd)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectError {
    #[error("failed to socket(): {0:?}")]
    Socket(Errno),
    #[error("failed to fcntl(FD_CLOEXEC): {0:?}")]
    SetCloexec(Errno),
    #[error("failed to ioctl(FIONBIO): {0:?}")]
    SwitchToNonBlocking(Errno),
    #[error("failed to connect(): {0:?}")]
    Connect(Errno),
}
