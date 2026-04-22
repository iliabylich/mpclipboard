use crate::{
    event_loop::EventLoopAwareOwnedFd,
    state::{Connected, Disconnected, State},
};

pub(crate) struct Connecting(pub(crate) EventLoopAwareOwnedFd);

impl Connecting {
    pub(crate) fn finish(self) -> State {
        match rustix::net::sockopt::socket_error(&self.0) {
            Ok(Ok(())) => State::Connected(Connected(self.0)),
            Ok(Err(err)) => {
                log::error!("{err:?}");
                State::Disconnected(Disconnected)
            }
            Err(err) => {
                log::error!("{err:?}");
                State::Disconnected(Disconnected)
            }
        }
    }
}
