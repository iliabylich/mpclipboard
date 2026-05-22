use crate::{
    Connectivity, Context, Output,
    state::{Connected, Handshaking, Ready},
};
use anyhow::{Context as _, Result};
use std::{
    net::TcpStream,
    os::fd::{AsRawFd, FromRawFd as _, IntoRawFd, OwnedFd},
};
use tungstenite::{HandshakeError, client::IntoClientRequest as _};

pub(crate) struct Established {
    fd: OwnedFd,
    started_at: u64,
}

impl Established {
    pub(crate) const fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            started_at: now,
        }
    }

    pub(crate) fn start_handshake(self, context: &Context) -> Result<(Connected, Option<Output>)> {
        log::trace!("starting handshake");

        let now = context.timer.now();

        let mut request = context
            .config
            .uri
            .clone()
            .into_client_request()
            .context("failed to create client request")?;

        let token = context.config.token.parse().context("non-ASCII token")?;
        request.headers_mut().insert("Token", token);

        let name = context.config.name.parse().context("non-ASCII name")?;
        request.headers_mut().insert("Name", name);

        let fd = self.fd.into_raw_fd();
        let stream = unsafe { TcpStream::from_raw_fd(fd) };
        let tls = context.tls.clone().0;

        match tungstenite::client_tls_with_config(request, stream, None, Some(tls)) {
            Ok((ws, response)) => {
                log::trace!("handshake completed: {}", response.status());
                Ok((
                    Connected::Ready(Ready::new(ws, fd, now)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                ))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("handshake interrupted");
                Ok((
                    Connected::Handshaking(Handshaking::new(handshake, fd, now)),
                    None,
                ))
            }
            Err(HandshakeError::Failure(err)) => Err(anyhow::anyhow!(err)),
        }
    }

    pub(crate) const fn should_disconnect_at(&self) -> u64 {
        self.started_at.wrapping_add(5)
    }
}

impl AsRawFd for Established {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
