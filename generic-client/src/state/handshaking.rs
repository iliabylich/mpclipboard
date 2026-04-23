use crate::{
    Connectivity, Output,
    event_loop::EventLoop,
    state::{Disconnected, Ready, State},
};
use std::{net::TcpStream, os::fd::AsRawFd, rc::Rc};
use tungstenite::{ClientHandshake, HandshakeError, stream::MaybeTlsStream};

/// cbindgen:ignore
type MidHandshake =
    tungstenite::handshake::MidHandshake<ClientHandshake<MaybeTlsStream<TcpStream>>>;

pub(crate) struct Handshaking {
    handshake_and_fd: Option<(MidHandshake, i32)>,
    event_loop: Rc<EventLoop>,
}

impl Handshaking {
    pub(crate) fn new(handshake: MidHandshake, fd: i32, event_loop: Rc<EventLoop>) -> Self {
        Self {
            handshake_and_fd: Some((handshake, fd)),
            event_loop,
        }
    }

    pub(crate) fn finish_handshake(&mut self) -> (State, Option<Output>) {
        let (handshake, rawfd) = self
            .handshake_and_fd
            .take()
            .expect("bug: malformed state in Handshaking");

        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                self.event_loop.remove(rawfd);
                return (
                    State::Disconnected(Disconnected),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Disconnected,
                    }),
                );
            }};
        }

        match handshake.handshake() {
            Ok((ws, response)) => {
                log::trace!("completed: {}", response.status());
                let event_loop = Rc::clone(&self.event_loop);
                (
                    State::Ready(Ready::new(ws, rawfd, event_loop)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                )
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("interrupted");
                let event_loop = Rc::clone(&self.event_loop);
                (
                    State::Handshaking(Handshaking::new(handshake, rawfd, event_loop)),
                    None,
                )
            }
            Err(HandshakeError::Failure(err)) => disconnect!(err),
        }
    }
}

impl AsRawFd for Handshaking {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.handshake_and_fd.as_ref().unwrap().1
    }
}
