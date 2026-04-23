use crate::{
    event_loop::EventLoop,
    state::{Connected, Disconnected, State},
};
use std::{
    os::fd::{AsRawFd, OwnedFd},
    rc::Rc,
};

pub(crate) struct Connecting {
    fd: Option<OwnedFd>,
    event_loop: Rc<EventLoop>,
}

impl Connecting {
    pub(crate) fn new(fd: OwnedFd, event_loop: Rc<EventLoop>) -> Self {
        Self {
            fd: Some(fd),
            event_loop,
        }
    }

    pub(crate) fn finish(&mut self) -> State {
        let fd = self.fd.take().expect("bug: malformed state in Connecting");
        let rawfd = fd.as_raw_fd();

        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                self.event_loop.remove(rawfd);
                return State::Disconnected(Disconnected);
            }};
        }

        match rustix::net::sockopt::socket_error(&fd) {
            Ok(Ok(())) => State::Connected(Connected::new(fd, Rc::clone(&self.event_loop))),
            Ok(Err(err)) => disconnect!(err),
            Err(err) => disconnect!(err),
        }
    }
}

impl AsRawFd for Connecting {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_ref().unwrap().as_raw_fd()
    }
}
