use crate::{
    Config,
    event_loop::{EventLoopAwareMidHandshake, EventLoopAwareOwnedFd, EventLoopAwareWebSocket},
    state::{Disconnected, Handshaking, Ready, State},
    tls::TLS,
};
use anyhow::Context as _;
use std::os::fd::AsRawFd as _;
use tungstenite::{HandshakeError, client::IntoClientRequest as _};

pub(crate) struct Connected(pub(crate) EventLoopAwareOwnedFd);

impl Connected {
    pub(crate) fn start_handshake(self, config: &Config, tls: &TLS) -> State {
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

        log::trace!("starting handshaking");

        let mut request = map_err_to_dead!(
            config
                .uri
                .clone()
                .into_client_request()
                .context("failed to create client request")
        );

        let token = map_err_to_dead!(config.token.parse().context("non-ASCII token"));
        request.headers_mut().insert("Token", token);

        let name = map_err_to_dead!(config.name.parse().context("non-ASCII name"));
        request.headers_mut().insert("Name", name);

        let event_loop = self.0.event_loop();
        let rawfd = self.0.as_raw_fd();
        let stream = self.0.into_tcp_stream();
        let tls = tls.clone().0;

        match tungstenite::client_tls_with_config(request, stream, None, Some(tls)) {
            Ok((ws, response)) => {
                log::trace!("handshake completed: {}", response.status());
                let ws = EventLoopAwareWebSocket::new(ws, rawfd, event_loop);
                State::Ready(Ready(ws))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("handshake interrupted");
                let mid_handshake = EventLoopAwareMidHandshake::new(handshake, rawfd, event_loop);
                State::Handshaking(Handshaking(mid_handshake))
            }
            Err(HandshakeError::Failure(err)) => {
                log::error!("{err:?}");
                State::Disconnected(Disconnected)
            }
        }
    }
}
