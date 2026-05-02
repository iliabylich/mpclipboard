use crate::{
    Connectivity, Output,
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
}

impl Handshaking {
    pub(crate) const fn new(handshake: MidHandshake, fd: i32) -> Self {
        Self { handshake, fd }
    }

    pub(crate) fn finish_handshake(self) -> Result<(Connected, Option<Output>)> {
        match self.handshake.handshake() {
            Ok((ws, response)) => {
                log::trace!("completed: {}", response.status());
                Ok((
                    Connected::Ready(Ready::new(ws, self.fd)),
                    Some(Output::ConnectivityChanged {
                        connectivity: Connectivity::Connected,
                    }),
                ))
            }
            Err(HandshakeError::Interrupted(handshake)) => {
                log::trace!("interrupted");
                Ok((Connected::Handshaking(Self::new(handshake, self.fd)), None))
            }
            Err(HandshakeError::Failure(err)) => Err(anyhow::anyhow!(err)),
        }
    }
}

impl AsRawFd for Handshaking {
    fn as_raw_fd(&self) -> i32 {
        self.fd
    }
}
