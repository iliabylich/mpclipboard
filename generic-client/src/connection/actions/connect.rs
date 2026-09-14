use anyhow::anyhow;
use mpclipboard_shared::prelude::*;
use rustix::{
    io::Errno,
    net::{AddressFamily, SocketType},
};
use std::{net::SocketAddrV4, os::fd::OwnedFd};

pub fn connect(addr: SocketAddrV4) -> Completion<OwnedFd, anyhow::Error, OwnedFd> {
    let fd = match rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None) {
        Ok(fd) => fd,
        Err(err) => return Failed(anyhow!("failed to socket(): {err:?}")),
    };
    #[cfg(target_os = "macos")]
    match rustix::net::sockopt::set_socket_nosigpipe(&fd, true) {
        Ok(()) => {}
        Err(err) => return Failed(anyhow!("failed to setsockopt(SO_NOSIGPIPE): {err:?}")),
    }

    if let Err(err) = rustix::io::ioctl_fionbio(&fd, true) {
        return Failed(anyhow!("failed to ioctl(): {err:?}"));
    }

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => Done(fd),
        Err(Errno::INPROGRESS) => Pending(fd),
        Err(err) => Failed(anyhow!("failed to connect(): {err:?}")),
    }
}
