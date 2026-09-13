use mpclipboard_shared::{
    Completion::{self, *},
    error,
};
use rustix::{
    io::Errno,
    net::{AddressFamily, SocketType},
};
use std::{net::SocketAddrV4, os::fd::OwnedFd};

pub fn connect(addr: SocketAddrV4) -> Completion<OwnedFd, OwnedFd> {
    let fd = match rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None) {
        Ok(fd) => fd,
        Err(err) => {
            error!("failed to socket(): {err:?}");
            return Failed;
        }
    };
    #[cfg(target_os = "macos")]
    match rustix::net::sockopt::set_socket_nosigpipe(&fd, true) {
        Ok(()) => {}
        Err(err) => {
            error!("failed to setsockopt(SO_NOSIGPIPE): {err:?}");
            return Failed;
        }
    }

    if let Err(err) = rustix::io::ioctl_fionbio(&fd, true) {
        error!("failed to ioctl(): {err:?}");
        return Failed;
    }

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => Done(fd),
        Err(Errno::INPROGRESS) => Pending(fd),
        Err(err) => {
            error!("{err:?}");
            Failed
        }
    }
}
