use crate::{
    Connectivity, Context, Output,
    event_loop::EventLoop,
    state::{Handshaking, Ready, State, StateVariant},
};
use anyhow::Context as _;
use std::{
    net::TcpStream,
    os::fd::{AsRawFd, FromRawFd as _, IntoRawFd, OwnedFd},
    rc::Rc,
};
use tungstenite::{HandshakeError, client::IntoClientRequest as _};

pub(crate) struct Connected {
    fd: OwnedFd,
    context: Context,
}

impl Connected {
    pub(crate) fn new(fd: OwnedFd, context: Context) -> Self {
        Self { fd, context }
    }

    fn start_handshake(self) -> (State, Option<Output>) {
        log::trace!("starting handshake");

        macro_rules! ok_or_disconnect {
            ($v:expr) => {
                match $v {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("{err:?}");
                        return self.disconnect();
                    }
                }
            };
        }

        log::trace!("starting handshaking");

        let mut request = ok_or_disconnect!(
            self.context
                .config
                .uri
                .clone()
                .into_client_request()
                .context("failed to create client request")
        );

        let token = ok_or_disconnect!(self.context.config.token.parse().context("non-ASCII token"));
        request.headers_mut().insert("Token", token);

        let name = ok_or_disconnect!(self.context.config.name.parse().context("non-ASCII name"));
        request.headers_mut().insert("Name", name);

        let fd = self.fd.into_raw_fd();
        let stream = unsafe { TcpStream::from_raw_fd(fd) };
        let tls = self.context.tls.clone().0;

        match tungstenite::client_tls_with_config(request, stream, None, Some(tls)) {
            Ok((ws, response)) => {
                log::trace!("handshake completed: {}", response.status());
                (
                    State::Ready(Ready::new(ws, fd, self.context)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                )
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("handshake interrupted");
                (
                    State::Handshaking(Handshaking::new(handshake, fd, self.context)),
                    None,
                )
            }
            Err(HandshakeError::Failure(err)) => {
                log::error!("{:?}", err);
                Self::disconnect_by(self.context, fd)
            }
        }
    }

    fn disconnect(self) -> (State, Option<Output>) {
        Self::disconnect_by(self.context, self.fd.as_raw_fd())
    }
}

impl AsRawFd for Connected {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}

impl StateVariant for Connected {
    fn tag(&self) -> &'static str {
        "Connected"
    }

    fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.context.event_loop)
    }

    fn transition(self, _readable: bool, _writable: bool) -> (State, Option<Output>) {
        self.start_handshake()
    }

    fn flip(self) -> (State, Option<Output>) {
        self.disconnect()
    }
}
