mod incoming;
mod outgoing;
mod plaintext;
mod read_half;
mod state;
mod step;
mod write_half;

pub(crate) use read_half::TlsReadHalf;
pub(crate) use write_half::TlsWriteHalf;

use crate::runtime::LoopContext;
use core::cell::RefCell;
use externally_driven_async_rt::{
    Context, State,
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use rustix::io::Errno;
use rustls::{
    ClientConfig,
    client::UnbufferedClientConnection,
    pki_types::{InvalidDnsNameError, ServerName},
    unbuffered::{EncodeError, EncryptError},
};
use state::TlsState;
use std::sync::Arc;
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

    pub(crate) async fn handshake(&mut self, ctx: &LoopContext<'_>) -> Result<(), TlsError> {
        let (rx, tx) = self.tcp.split();
        let rx = TlsReadHalf::new(rx, &self.state);
        let tx = TlsWriteHalf::new(tx, &self.state);
        loop {
            let step = self.state.borrow_mut().step()?;
            match step {
                Step::Writable => return Ok(()),
                Step::Transmit => tx.transmit(ctx).await?,
                Step::Blocked => rx.fill(ctx).await?,
                Step::Plaintext => return Err(TlsError::PlaintextDuringHandshake),
                Step::PeerClosed => return Err(TlsError::Closed),
            }
        }
    }
}

impl AsyncRead for TlsStream {
    type Error = TlsError;

    async fn read<
        'a,
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &'a mut self,
        ctx: &'a Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
        buf: &'a mut [u8],
    ) -> Result<usize, TlsError> {
        let (mut rx, _) = self.split();
        rx.read(ctx, buf).await
    }
}

impl AsyncWrite for TlsStream {
    type Error = TlsError;

    async fn write<
        'a,
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &'a mut self,
        ctx: &'a Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
        buf: &'a [u8],
    ) -> Result<usize, TlsError> {
        let (_, mut tx) = self.split();
        tx.write(ctx, buf).await
    }

    async fn flush<
        'a,
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &'a mut self,
        ctx: &'a Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
    ) -> Result<(), TlsError> {
        let (_, mut tx) = self.split();
        tx.flush(ctx).await
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TlsError {
    #[error("invalid server name: {0}")]
    InvalidServerName(#[from] InvalidDnsNameError),
    #[error("TLS error: {0}")]
    Rustls(#[from] rustls::Error),
    #[error("failed to encode TLS data: {0}")]
    Encode(EncodeError),
    #[error("failed to encrypt TLS data: {0}")]
    Encrypt(EncryptError),
    #[error("unexpected TLS state {0}")]
    UnexpectedState(String),
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
