mod incoming;
mod outgoing;
mod plaintext;
mod read_half;
mod state;
mod step;
mod write_half;

pub(crate) use read_half::TlsReadHalf;
pub(crate) use write_half::TlsWriteHalf;

use crate::runtime::TcpStream;
use alloc::sync::Arc;
use core::cell::RefCell;
use externally_driven_async_rt::io::{AsyncRead, AsyncWrite};
use rustix::io::Errno;
use rustls::{
    ClientConfig,
    client::UnbufferedClientConnection,
    pki_types::{InvalidDnsNameError, ServerName},
    unbuffered::{EncodeError, EncryptError},
};
use state::TlsState;
use step::Step;

pub(crate) struct TlsStream {
    tcp: TcpStream,
    state: RefCell<TlsState>,
}

impl TlsStream {
    pub(crate) fn new(
        tcp: TcpStream,
        config: Arc<ClientConfig>,
        host: &str,
    ) -> Result<Self, TlsError> {
        let name = ServerName::try_from(host.to_string())?;
        let conn = UnbufferedClientConnection::new(config, name)?;

        Ok(Self {
            tcp,
            state: RefCell::new(TlsState::new(conn)),
        })
    }

    pub(crate) fn split(&mut self) -> (TlsReadHalf<'_>, TlsWriteHalf<'_>) {
        let (rx, tx) = self.tcp.split();
        (
            TlsReadHalf::new(rx, &self.state),
            TlsWriteHalf::new(tx, &self.state),
        )
    }

    pub(crate) async fn handshake(&mut self) -> Result<(), TlsError> {
        let (rx, tx) = self.tcp.split();
        let mut rx = TlsReadHalf::new(rx, &self.state);
        let mut tx = TlsWriteHalf::new(tx, &self.state);
        loop {
            let step = self.state.borrow_mut().step()?;
            match step {
                Step::Writable => return Ok(()),
                Step::Transmit => tx.transmit().await?,
                Step::Blocked => rx.fill().await?,
                Step::Plaintext => return Err(TlsError::PlaintextDuringHandshake),
                Step::PeerClosed => return Err(TlsError::Closed),
            }
        }
    }
}

impl AsyncRead for TlsStream {
    type Error = TlsError;

    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize, TlsError> {
        let (mut rx, _) = self.split();
        rx.read(buf).await
    }
}

impl AsyncWrite for TlsStream {
    type Error = TlsError;

    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> Result<usize, TlsError> {
        let (_, mut tx) = self.split();
        tx.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), TlsError> {
        let (_, mut tx) = self.split();
        tx.flush().await
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TlsError {
    #[error("invalid server name: {0}")]
    InvalidServerName(InvalidDnsNameError),
    #[error("TLS error: {0}")]
    Rustls(rustls::Error),
    #[error("failed to encode TLS data: {0}")]
    Encode(EncodeError),
    #[error("failed to encrypt TLS data: {0}")]
    Encrypt(EncryptError),
    #[error("unexpected TLS state")]
    UnexpectedState,
    #[error("received application data during TLS handshake")]
    PlaintextDuringHandshake,
    #[error("can't write before TLS handshake is complete")]
    HandshakeNotComplete,
    #[error("incoming TLS buffer is full")]
    IncomingBufferFull,
    #[error("plaintext buffer is full")]
    PlaintextBufferFull,
    #[error("failed to read from TCP socket: {0}")]
    Read(Errno),
    #[error("failed to write to TCP socket: {0}")]
    Write(Errno),
    #[error("TCP socket accepted zero bytes")]
    WriteZero,
    #[error("connection closed")]
    Closed,
}

impl From<InvalidDnsNameError> for TlsError {
    fn from(err: InvalidDnsNameError) -> Self {
        Self::InvalidServerName(err)
    }
}

impl From<rustls::Error> for TlsError {
    fn from(err: rustls::Error) -> Self {
        Self::Rustls(err)
    }
}
