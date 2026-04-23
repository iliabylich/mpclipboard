use crate::{
    Config,
    event_loop::EventLoop,
    state::{Disconnected, Handshaking, Ready, State},
    tls::TLS,
};
use anyhow::Context as _;
use std::{
    net::TcpStream,
    os::fd::{AsRawFd, FromRawFd as _, IntoRawFd, OwnedFd},
    rc::Rc,
};
use tungstenite::{HandshakeError, client::IntoClientRequest as _};

pub(crate) struct Connected {
    fd: Option<OwnedFd>,
    event_loop: Rc<EventLoop>,
}

impl Connected {
    pub(crate) fn new(fd: OwnedFd, event_loop: Rc<EventLoop>) -> Self {
        Self {
            fd: Some(fd),
            event_loop,
        }
    }

    pub(crate) fn start_handshake(&mut self, config: &Config, tls: &TLS) -> State {
        let fd = self.fd.take().expect("bug: malformed state in Connected");
        let rawfd = fd.as_raw_fd();

        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                self.event_loop.remove(rawfd);
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

        log::trace!("starting handshaking");

        let mut request = ok_or_disconnect!(
            config
                .uri
                .clone()
                .into_client_request()
                .context("failed to create client request")
        );

        let token = ok_or_disconnect!(config.token.parse().context("non-ASCII token"));
        request.headers_mut().insert("Token", token);

        let name = ok_or_disconnect!(config.name.parse().context("non-ASCII name"));
        request.headers_mut().insert("Name", name);

        let stream = unsafe { TcpStream::from_raw_fd(fd.into_raw_fd()) };
        let tls = tls.clone().0;

        match tungstenite::client_tls_with_config(request, stream, None, Some(tls)) {
            Ok((ws, response)) => {
                log::trace!("handshake completed: {}", response.status());
                let event_loop = Rc::clone(&self.event_loop);
                State::Ready(Ready::new(ws, rawfd, event_loop))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("handshake interrupted");
                let event_loop = Rc::clone(&self.event_loop);
                State::Handshaking(Handshaking::new(handshake, rawfd, event_loop))
            }
            Err(HandshakeError::Failure(err)) => {
                log::error!("{err:?}");
                self.event_loop.remove(rawfd);
                State::Disconnected(Disconnected)
            }
        }
    }
}

impl AsRawFd for Connected {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_ref().unwrap().as_raw_fd()
    }
}
