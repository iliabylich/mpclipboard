use crate::{
    config::Config,
    connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream},
    url_ext::UrlExt,
};
use mpclipboard_shared::prelude::*;
use rustix::{
    fd::OwnedFd,
    io::{Errno, FdFlags},
    net::{AddressFamily, SocketType},
};

type FdAndMaybeTlsStream = (OwnedFd, MaybeTlsStream);

pub fn reconnect(
    config: &Config,
) -> Result<Completion<FdAndMaybeTlsStream, FdAndMaybeTlsStream>, ConnectionError> {
    let addr = config
        .url
        .resolve()
        .map_err(ConnectionError::FailedToResolveUrl)?;

    let stream =
        MaybeTlsStream::new(&config.url).map_err(ConnectionError::FailedToCreateTlsStream)?;

    let fd = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)
        .map_err(ConnectionError::FailedToSocket)?;

    rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectionError::FailedToSetCloexec)?;

    #[cfg(target_os = "macos")]
    rustix::net::sockopt::set_socket_nosigpipe(&fd, true)
        .map_err(ConnectionError::FailedToSetNoSigPipe)?;

    rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectionError::FailedToSwitchToNonBlocking)?;

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => Ok(Done((fd, stream))),
        Err(Errno::INPROGRESS) => Ok(Pending((fd, stream))),
        Err(errno) => Err(ConnectionError::FailedToConnect(errno)),
    }
}
