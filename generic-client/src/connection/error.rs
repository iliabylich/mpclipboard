use crate::connection::{
    actions::{
        ConnectError, FinishConnectingError, FinishTlsHandshakeError, ReadDnsResponseError,
        ReadMessageError, ReadUpgradeResponseError, StartResolvingError, WriteDnsRequestError,
        WriteMessageError, WriteUpgradeRequestError,
    },
    maybe_tls_stream::MaybeTlsStreamError,
};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("failed to start resolving: {0}")]
    StartResolving(#[from] StartResolvingError),
    #[error("failed to write DNS request: {0}")]
    WriteDnsRequest(#[from] WriteDnsRequestError),
    #[error("failed to read DNS response: {0}")]
    ReadDnsResponse(#[from] ReadDnsResponseError),
    #[error("failed to connect: {0}")]
    Connect(#[from] ConnectError),
    #[error("failed to finish connecting: {0}")]
    FinishConnecting(#[from] FinishConnectingError),
    #[error("failed to finish TLS handshake: {0}")]
    FinishTlsHandshake(#[from] FinishTlsHandshakeError),
    #[error("failed to write upgrade request: {0}")]
    WriteUpgradeRequest(#[from] WriteUpgradeRequestError),
    #[error("failed to read upgrade response: {0}")]
    ReadUpgradeResponse(#[from] ReadUpgradeResponseError),
    #[error("failed to read message: {0}")]
    ReadMessage(#[from] ReadMessageError),
    #[error("failed to write message: {0}")]
    WriteMessage(#[from] WriteMessageError),
    #[error("failed to flush TLS data: {0}")]
    FlushTls(#[from] MaybeTlsStreamError),
}
