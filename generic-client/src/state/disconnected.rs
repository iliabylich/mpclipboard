use crate::{
    event_loop::EventLoop,
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
        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                return State::Disconnected(Disconnected);
            }};
        }

        macro_rules! ok_or_disconnect {
            ($v:expr) => {
                match $v {
                    Ok(v) => v,
                    Err(err) => disconnect!(err),
                }
            };
        }

        log::trace!("starting connect");

        let domain = if remote_addr.is_ipv4() {
            AddressFamily::INET
        } else {
            AddressFamily::INET6
        };

        let fd = ok_or_disconnect!(
            rustix::net::socket(domain, SocketType::STREAM, None).context("socket()")
        );

        let flags = ok_or_disconnect!(rustix::io::fcntl_getfd(&fd).context("F_GETFD()"));
        ok_or_disconnect!(
            rustix::io::fcntl_setfd(&fd, flags | FdFlags::CLOEXEC).context("F_SETFD(FD_CLOEXEC)")
        );

        let flags = ok_or_disconnect!(rustix::fs::fcntl_getfl(&fd).context("F_GETFL()"));
        ok_or_disconnect!(
            rustix::fs::fcntl_setfl(&fd, flags | OFlags::NONBLOCK).context("F_SETFL(O_NONBLOCK)")
        );

        let connected = match rustix::net::connect(&fd, &remote_addr) {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == rustix::io::Errno::INPROGRESS.raw_os_error() => false,
            Err(err) => disconnect!(err),
        };

        let (fd, state) = if connected {
            log::trace!("connected; fd: {}", fd.as_raw_fd());
            (
                fd.as_raw_fd(),
                State::Connected(Connected::new(fd, Rc::clone(&event_loop))),
            )
        } else {
            log::trace!("connecting; fd: {}", fd.as_raw_fd());
            (
                fd.as_raw_fd(),
                State::Connecting(Connecting::new(fd, Rc::clone(&event_loop))),
            )
        };

        if let Err(err) = event_loop.add(fd, true, true) {
            disconnect!(err);
        }

        state
    }
}
