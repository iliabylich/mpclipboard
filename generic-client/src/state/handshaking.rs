use crate::{
    Connectivity, Context, Output,
    event_loop::EventLoop,
    state::{Ready, State, StateVariant},
};
use std::{net::TcpStream, os::fd::AsRawFd, rc::Rc};
use tungstenite::{ClientHandshake, HandshakeError, stream::MaybeTlsStream};

/// cbindgen:ignore
type MidHandshake =
    tungstenite::handshake::MidHandshake<ClientHandshake<MaybeTlsStream<TcpStream>>>;

pub(crate) struct Handshaking {
    handshake: MidHandshake,
    fd: i32,
    context: Context,
}

impl Handshaking {
    pub(crate) fn new(handshake: MidHandshake, fd: i32, context: Context) -> Self {
        Self {
            handshake,
            fd,
            context,
        }
    }

    fn finish_handshake(self) -> (State, Option<Output>) {
        match self.handshake.handshake() {
            Ok((ws, response)) => {
                log::trace!("completed: {}", response.status());
                (
                    State::Ready(Ready::new(ws, self.fd, self.context)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                )
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("interrupted");
                (
                    State::Handshaking(Handshaking::new(handshake, self.fd, self.context)),
                    None,
                )
            }
            Err(HandshakeError::Failure(err)) => {
                log::error!("{err:?}");
                Self::disconnect_by(self.context, self.fd)
            }
        }
    }
}

impl AsRawFd for Handshaking {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd
    }
}

impl StateVariant for Handshaking {
    fn tag(&self) -> &'static str {
        "Handshaking"
    }

    fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.context.event_loop)
    }

    fn transition(self, _readable: bool, _writable: bool) -> (State, Option<Output>) {
        self.finish_handshake()
    }

    fn flip(self) -> (State, Option<Output>) {
        Self::disconnect_by(self.context, self.fd)
    }
}
