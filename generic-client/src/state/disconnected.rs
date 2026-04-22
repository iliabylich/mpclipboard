use crate::{
    event_loop::{EventLoop, EventLoopAwareOwnedFd},
    state::{Connected, Connecting, State},
};
use anyhow::Context as _;
use rustix::{
    fs::OFlags,
    io::FdFlags,
    net::{AddressFamily, SocketType},
};
use std::{net::SocketAddr, os::fd::AsRawFd as _, rc::Rc};

#[derive(Clone, Copy)]
pub(crate) struct Disconnected;

impl Disconnected {
    pub(crate) fn connect(remote_addr: SocketAddr, event_loop: Rc<EventLoop>) -> State {
        macro_rules! map_err_to_dead {
            ($v:expr) => {
                match $v {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("{err:?}");
                        return State::Disconnected(Disconnected);
                    }
                }
            };
        }
        log::trace!("starting connect");

        let domain = if remote_addr.is_ipv4() {
            AddressFamily::INET
        } else {
            AddressFamily::INET6
        };

        let fd = map_err_to_dead!(
            rustix::net::socket(domain, SocketType::STREAM, None).context("socket()")
        );

        let flags = map_err_to_dead!(rustix::io::fcntl_getfd(&fd).context("F_GETFD()"));
        map_err_to_dead!(
            rustix::io::fcntl_setfd(&fd, flags | FdFlags::CLOEXEC).context("F_SETFD(FD_CLOEXEC)")
        );

        let flags = map_err_to_dead!(rustix::fs::fcntl_getfl(&fd).context("F_GETFL()"));
        map_err_to_dead!(
            rustix::fs::fcntl_setfl(&fd, flags | OFlags::NONBLOCK).context("F_SETFL(O_NONBLOCK)")
        );

        let connected = match rustix::net::connect(&fd, &remote_addr) {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == rustix::io::Errno::INPROGRESS.raw_os_error() => false,
            Err(err) => {
                log::error!("{err:?}");
                return State::Disconnected(Disconnected);
            }
        };

        let (fd, state) = if connected {
            let fd = EventLoopAwareOwnedFd::new(fd, Rc::clone(&event_loop));
            log::trace!("connected; fd: {}", fd.as_raw_fd());
            (fd.as_raw_fd(), State::Connected(Connected(fd)))
        } else {
            let fd = EventLoopAwareOwnedFd::new(fd, Rc::clone(&event_loop));
            log::trace!("connecting; fd: {}", fd.as_raw_fd());
            (fd.as_raw_fd(), State::Connecting(Connecting(fd)))
        };

        if let Err(err) = event_loop.add(fd, true, true) {
            log::error!("{err:?}");
            return State::Disconnected(Disconnected);
        }

        state
    }
}
