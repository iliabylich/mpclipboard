use crate::{
    Connectivity, Context, Output,
    event_loop::EventLoop,
    state::{Connected, Connecting, State, StateVariant},
};
use anyhow::Context as _;
use rustix::{
    fs::OFlags,
    io::FdFlags,
    net::{AddressFamily, SocketType},
};
use std::{os::fd::AsRawFd as _, rc::Rc};

pub(crate) struct Disconnected {
    context: Context,
}

impl Disconnected {
    pub(crate) fn new(context: Context) -> Self {
        Self { context }
    }

    fn connect(self) -> (State, Option<Output>) {
        log::trace!("connect");

        let context = self.context;
        let event_loop = Rc::clone(&context.event_loop);

        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                return (State::Disconnected(Disconnected::new(context)), None);
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

        let domain = if context.remote_addr.is_ipv4() {
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

        let connected = match rustix::net::connect(&fd, &context.remote_addr) {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == rustix::io::Errno::INPROGRESS.raw_os_error() => false,
            Err(err) => disconnect!(err),
        };

        let rawfd = fd.as_raw_fd();

        let state = if connected {
            log::trace!("connected; fd: {rawfd}");
            State::Connected(Connected::new(fd, context))
        } else {
            log::trace!("connecting; fd: {rawfd}");
            State::Connecting(Connecting::new(fd, context))
        };

        event_loop.add(rawfd, true, true);

        (
            state,
            Some(Output::ConnectivityChanged {
                connectivity: Connectivity::Connecting,
            }),
        )
    }
}

impl StateVariant for Disconnected {
    fn tag(&self) -> &'static str {
        "Disconnected"
    }

    fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.context.event_loop)
    }

    fn transition(self, _readable: bool, _writable: bool) -> (State, Option<Output>) {
        self.connect()
    }

    fn flip(self) -> (State, Option<Output>) {
        self.connect()
    }
}
