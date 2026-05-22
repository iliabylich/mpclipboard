use crate::{
    Connectivity, Context, Output,
    state::{Connected, Ready},
};
use anyhow::Result;
use std::{net::TcpStream, os::fd::AsRawFd};
use tungstenite::{ClientHandshake, HandshakeError, stream::MaybeTlsStream};

/// cbindgen:ignore
type MidHandshake =
    tungstenite::handshake::MidHandshake<ClientHandshake<MaybeTlsStream<TcpStream>>>;

pub(crate) struct Handshaking {
    handshake: MidHandshake,
    fd: i32,
    started_at: u64,
}

impl Handshaking {
    pub(crate) const fn new(handshake: MidHandshake, fd: i32, now: u64) -> Self {
        Self {
            handshake,
            fd,
            started_at: now,
        }
    }

    pub(crate) fn finish_handshake(self, context: &Context) -> Result<(Connected, Option<Output>)> {
        let now = context.timer.now();

        match self.handshake.handshake() {
            Ok((ws, response)) => {
                log::trace!("completed: {}", response.status());
                Ok((
                    Connected::Ready(Ready::new(ws, self.fd, now)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                ))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("interrupted");
                Ok((
                    Connected::Handshaking(Self::new(handshake, self.fd, self.started_at)),
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

impl AsRawFd for Handshaking {
    fn as_raw_fd(&self) -> i32 {
        self.fd
    }
}
