use crate::{
    Context, Output,
    event_loop::EventLoop,
    state::{Connected, State, StateVariant},
};
use std::{
    os::fd::{AsRawFd, OwnedFd},
    rc::Rc,
};

pub(crate) struct Connecting {
    fd: OwnedFd,
    context: Context,
}

impl Connecting {
    pub(crate) fn new(fd: OwnedFd, context: Context) -> Self {
        Self { fd, context }
    }

    fn finish_connecting(self) -> (State, Option<Output>) {
        log::trace!("finish connecting");

        match rustix::net::sockopt::socket_error(&self.fd) {
            Ok(Ok(())) => {
                self.context.event_loop.modify(self.as_raw_fd(), true, true);

                (
                    State::Connected(Connected::new(self.fd, self.context)),
                    None,
                )
            }
            Ok(Err(err)) | Err(err) => {
                log::error!("{err:?}");
                self.disconnect()
            }
        }
    }

    fn disconnect(self) -> (State, Option<Output>) {
        Self::disconnect_by(self.context, self.fd.as_raw_fd())
    }
}

impl AsRawFd for Connecting {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}

impl StateVariant for Connecting {
    fn tag(&self) -> &'static str {
        "Connecting"
    }

    fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.context.event_loop)
    }

    fn transition(self, _readable: bool, _writable: bool) -> (State, Option<Output>) {
        self.finish_connecting()
    }

    fn flip(self) -> (State, Option<Output>) {
        self.disconnect()
    }
}
