use crate::{
    config::Config,
    connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream},
};
use mpclipboard_shared::prelude::*;
use rustix::{
    io::Errno,
    net::{AddressFamily, SocketType},
};
use std::os::fd::OwnedFd;

pub fn reconnect(
    config: &Config,
) -> Completion<(OwnedFd, MaybeTlsStream), ConnectionError, (OwnedFd, MaybeTlsStream)> {
    let addr = match config.url.resolve() {
        Ok(addr) => addr,
        Err(err) => return Failed(ConnectionError::FailedToResolveUrl(err)),
    };

    let stream = match MaybeTlsStream::new(&config.url) {
        Ok(stream) => stream,
        Err(err) => return Failed(ConnectionError::FailedToCreateTlsStream(err)),
    };

    let fd = match rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None) {
        Ok(fd) => fd,
        Err(errno) => return Failed(ConnectionError::FailedToSocket(errno)),
    };
    #[cfg(target_os = "macos")]
    match rustix::net::sockopt::set_socket_nosigpipe(&fd, true) {
        Ok(()) => {}
        Err(errno) => return Failed(ConnectionError::FailedToSetNoSigPipe(errno)),
    }

    if let Err(errno) = rustix::io::ioctl_fionbio(&fd, true) {
        return Failed(ConnectionError::FailedToSwitchToNonBlocking(errno));
    }

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => Done((fd, stream)),
        Err(Errno::INPROGRESS) => Pending((fd, stream)),
        Err(errno) => Failed(ConnectionError::FailedToConnect(errno)),
    }
}
