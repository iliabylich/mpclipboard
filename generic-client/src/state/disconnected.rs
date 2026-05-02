use crate::{
    Connectivity, Context, Output,
    state::{Connected, Connection, Established, Establishing},
};
use anyhow::{Context as _, Result};
use rustix::{
    fs::OFlags,
    io::FdFlags,
    net::{AddressFamily, SocketType},
};
use std::os::fd::AsRawFd as _;

#[derive(Default)]
pub(crate) struct Disconnected;

impl Disconnected {
    fn try_connect(context: &Context) -> Result<(Connected, Option<Output>)> {
        log::trace!("connect");

        let domain = if context.remote_addr.is_ipv4() {
            AddressFamily::INET
        } else {
            AddressFamily::INET6
        };

        let fd = rustix::net::socket(domain, SocketType::STREAM, None).context("socket()")?;

        let flags = rustix::io::fcntl_getfd(&fd).context("F_GETFD()")?;
        rustix::io::fcntl_setfd(&fd, flags | FdFlags::CLOEXEC).context("F_SETFD(FD_CLOEXEC)")?;

        let flags = rustix::fs::fcntl_getfl(&fd).context("F_GETFL()")?;
        rustix::fs::fcntl_setfl(&fd, flags | OFlags::NONBLOCK).context("F_SETFL(O_NONBLOCK)")?;

        let connected = match rustix::net::connect(&fd, &context.remote_addr) {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == rustix::io::Errno::INPROGRESS.raw_os_error() => false,
            Err(err) => return Err(anyhow::anyhow!(err)),
        };

        let rawfd = fd.as_raw_fd();

        let state = if connected {
            log::trace!("connected; fd: {rawfd}");
            Connected::Established(Established::new(fd))
        } else {
            log::trace!("connecting; fd: {rawfd}");
            Connected::Establishing(Establishing::new(fd))
        };

        context.event_loop.add(rawfd, true, true)?;

        Ok((
            state,
            Some(Output::ConnectivityChanged {
                connectivity: Connectivity::Connecting,
            }),
        ))
    }

    pub(crate) fn connect(context: &Context) -> (Connection, Option<Output>) {
        match Self::try_connect(context) {
            Ok((conn, output)) => (Connection::Connected(Box::new(conn)), output),
            Err(err) => {
                log::error!("{err:?}");
                (Connection::Disconnected, None)
            }
        }
    }

    pub(crate) const fn tag() -> &'static str {
        "Disconnected"
    }
}
