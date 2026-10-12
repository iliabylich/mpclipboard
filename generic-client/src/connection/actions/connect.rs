use crate::{
    config::Config,
    connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError},
};
use core::net::SocketAddr;
use mpclipboard_shared::prelude::*;
use rustix::{
    fd::OwnedFd,
    io::{Errno, FdFlags},
    net::{AddressFamily, SocketType},
};

type FdAndMaybeTlsStream = (OwnedFd, MaybeTlsStream);

pub fn connect(
    addr: SocketAddr,
    config: &Config,
) -> Result<Completion<FdAndMaybeTlsStream, FdAndMaybeTlsStream>, ConnectError> {
    let stream = MaybeTlsStream::new(&config.url)?;

    let fd = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)
        .map_err(ConnectError::Socket)?;

    rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectError::SetCloexec)?;

    #[cfg(target_os = "macos")]
    rustix::net::sockopt::set_socket_nosigpipe(&fd, true).map_err(ConnectError::SetNoSigPipe)?;

    rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectError::SwitchToNonBlocking)?;

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => Ok(Done((fd, stream))),
        Err(Errno::INPROGRESS) => Ok(Pending((fd, stream))),
        Err(errno) => Err(ConnectError::Connect(errno)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("failed to create TLS stream: {0}")]
    CreateTlsStream(#[from] MaybeTlsStreamError),
    #[error("failed to socket(): {0:?}")]
    Socket(Errno),
    #[error("failed to fcntl(FD_CLOEXEC): {0:?}")]
    SetCloexec(Errno),
    #[cfg(target_os = "macos")]
    #[error("failed to setsockopt(SO_NOSIGPIPE): {0:?}")]
    SetNoSigPipe(Errno),
    #[error("failed to ioctl(FIONBIO): {0:?}")]
    SwitchToNonBlocking(Errno),
    #[error("failed to connect(): {0:?}")]
    Connect(Errno),
}
