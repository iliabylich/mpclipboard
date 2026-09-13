use mpclipboard_shared::error;
use rustix::{
    io::Errno,
    net::{AddressFamily, SocketType},
};
use std::{net::SocketAddrV4, os::fd::OwnedFd};

pub fn connect(addr: SocketAddrV4) -> ConnectResult {
    let fd = match rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None) {
        Ok(fd) => fd,
        Err(err) => {
            error!("failed to socket(): {err:?}");
            return ConnectResult::Failed;
        }
    };
    #[cfg(target_os = "macos")]
    match rustix::net::sockopt::set_socket_nosigpipe(&fd, true) {
        Ok(()) => {}
        Err(err) => {
            error!("failed to setsockopt(SO_NOSIGPIPE): {err:?}");
            return ConnectResult::Failed;
        }
    }

    if let Err(err) = rustix::io::ioctl_fionbio(&fd, true) {
        error!("failed to ioctl(): {err:?}");
        return ConnectResult::Failed;
    }

    match rustix::net::connect(&fd, &addr) {
        Ok(()) => ConnectResult::Connected(fd),
        Err(Errno::INPROGRESS) => ConnectResult::StillPending(fd),
        Err(err) => {
            error!("{err:?}");
            ConnectResult::Failed
        }
    }
}

pub enum ConnectResult {
    Connected(OwnedFd),
    StillPending(OwnedFd),
    Failed,
}
