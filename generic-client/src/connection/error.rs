use crate::connection::maybe_tls_stream::MaybeTlsStreamError;
use mpclipboard_shared::{
    MessageError, MessageWriterError, TcpKeepAliveError, UpgradeRequestWriterError,
    UpgradeResponseReaderError, UrlResolveError,
};
use rustix::io::Errno;

#[derive(Debug)]
pub enum ConnectionError {
    FailedToResolveUrl(UrlResolveError),
    FailedToCreateTlsStream(MaybeTlsStreamError),
    FailedToSocket(Errno),
    #[cfg(target_os = "macos")]
    FailedToSetNoSigPipe(Errno),
    FailedToSwitchToNonBlocking(Errno),
    FailedToConnect(Errno),
    FailedToFinishTlsHandshake(MaybeTlsStreamError),
    FailedToWrite(MaybeTlsStreamError),
    UpgradeRequestWriterError(UpgradeRequestWriterError),
    FailedToRead(MaybeTlsStreamError),
    UpgradeResponseReaderError(UpgradeResponseReaderError),
    TcpKeepAliveError(TcpKeepAliveError),
    MessageError(MessageError),
    FailedToFlushTls(MaybeTlsStreamError),
    MessageWriterError(MessageWriterError),
}

impl core::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FailedToResolveUrl(err) => write!(f, "failed to resolve URL: {err}"),
            Self::FailedToCreateTlsStream(err) => write!(f, "failed to create TLS stream: {err}"),
            Self::FailedToSocket(errno) => write!(f, "failed to socket(): {errno:?}"),
            #[cfg(target_os = "macos")]
            Self::FailedToSetNoSigPipe(errno) => {
                write!(f, "failed to setsockopt(SO_NOSIGPIPE): {errno:?}")
            }
            Self::FailedToSwitchToNonBlocking(errno) => {
                write!(f, "failed to ioctl(FIONBIO): {errno:?}")
            }
            Self::FailedToConnect(errno) => write!(f, "failed to connect(): {errno:?}"),
            Self::FailedToFinishTlsHandshake(err) => {
                write!(f, "failed to finish TLS handshake: {err}")
            }
            Self::FailedToWrite(err) => write!(f, "failed to write: {err}"),
            Self::UpgradeRequestWriterError(err) => write!(f, "UpgradeRequestWriter error: {err}"),
            Self::FailedToRead(err) => write!(f, "failed to read: {err}"),
            Self::UpgradeResponseReaderError(err) => {
                write!(f, "UpgradeResponseReader error: {err}")
            }
            Self::TcpKeepAliveError(err) => write!(f, "TCP keepalive error: {err}"),
            Self::MessageError(err) => write!(f, "message error: {err}"),
            Self::FailedToFlushTls(err) => write!(f, "failed to flush TLS data: {err}"),
            Self::MessageWriterError(err) => write!(f, "MessageWriter error: {err}"),
        }
    }
}

impl core::error::Error for ConnectionError {}
