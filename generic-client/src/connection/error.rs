use crate::{connection::maybe_tls_stream::MaybeTlsStreamError, url_ext::UrlResolveError};
use mpclipboard_shared::{
    MessageError, MessageWriterError, TcpKeepAliveError, UpgradeRequestWriterError,
    UpgradeResponseReaderError,
};
use rustix::io::Errno;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("failed to resolve URL: {0}")]
    FailedToResolveUrl(#[from] UrlResolveError),
    #[error("failed to create TLS stream: {0}")]
    FailedToCreateTlsStream(MaybeTlsStreamError),
    #[error("failed to socket(): {0:?}")]
    FailedToSocket(Errno),
    #[error("failed to fcntl(FD_CLOEXEC): {0:?}")]
    FailedToSetCloexec(Errno),
    #[cfg(target_os = "macos")]
    #[error("failed to setsockopt(SO_NOSIGPIPE): {0:?}")]
    FailedToSetNoSigPipe(Errno),
    #[error("failed to ioctl(FIONBIO): {0:?}")]
    FailedToSwitchToNonBlocking(Errno),
    #[error("failed to connect(): {0:?}")]
    FailedToConnect(Errno),
    #[error("failed to finish TLS handshake: {0}")]
    FailedToFinishTlsHandshake(MaybeTlsStreamError),
    #[error("failed to write: {0}")]
    FailedToWrite(MaybeTlsStreamError),
    #[error("UpgradeRequestWriter error: {0}")]
    UpgradeRequestWriterError(#[from] UpgradeRequestWriterError),
    #[error("failed to read: {0}")]
    FailedToRead(MaybeTlsStreamError),
    #[error("UpgradeResponseReader error: {0}")]
    UpgradeResponseReaderError(#[from] UpgradeResponseReaderError),
    #[error("TCP keepalive error: {0}")]
    TcpKeepAliveError(#[from] TcpKeepAliveError),
    #[error("message error: {0}")]
    MessageError(#[from] MessageError),
    #[error("failed to flush TLS data: {0}")]
    FailedToFlushTls(MaybeTlsStreamError),
    #[error("MessageWriter error: {0}")]
    MessageWriterError(#[from] MessageWriterError),
}
