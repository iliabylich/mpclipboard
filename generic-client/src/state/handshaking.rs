use crate::{
    event_loop::{EventLoopAwareMidHandshake, EventLoopAwareWebSocket},
    state::{Disconnected, Ready, State},
};
use std::os::fd::AsRawFd as _;
use tungstenite::HandshakeError;

pub(crate) struct Handshaking(pub(crate) EventLoopAwareMidHandshake);

impl Handshaking {
    pub(crate) fn finish_handshake(self) -> State {
        let fd = self.0.as_raw_fd();
        let event_loop = self.0.event_loop();

        match self.0.handshake() {
            Ok((ws, response)) => {
                log::trace!("completed: {}", response.status());
                let ws = EventLoopAwareWebSocket::new(ws, fd, event_loop);
                State::Ready(Ready(ws))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("interrupted");
                let mid_handshake = EventLoopAwareMidHandshake::new(handshake, fd, event_loop);
                State::Handshaking(Handshaking(mid_handshake))
            }
            Err(HandshakeError::Failure(err)) => {
                log::error!("{err:?}");
                State::Disconnected(Disconnected)
            }
        }
    }
}
