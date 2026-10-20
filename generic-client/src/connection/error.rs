use mini_sansio_dns::DnsError;
use mpclipboard_shared::{MessageDecodeError, MessageWriterError};
use quinn_proto::{ReadableError, WriteError};
use rustix::io::Errno;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionError {
    // socket-related error
    #[error("failed to socket(): {0:?}")]
    OpenSocket(Errno),
    #[error("failed to fcntl(FD_CLOEXEC): {0:?}")]
    SetCloexec(Errno),
    #[error("failed to ioctl(FIONBIO): {0:?}")]
    SwitchToNonBlocking(Errno),

    // DNS-related errors
    #[error("failed to connect() to DNS server: {0:?}")]
    ConnectToDns(Errno),
    #[error("failed to send() DNS request: {0:?}")]
    WriteDnsRequest(Errno),
    #[error("failed to recv() DNS response: {0:?}")]
    ReadDnsResponse(Errno),
    #[error("DNS error: {0}")]
    ParseDnsError(#[from] DnsError),

    // app connection -level errors
    #[error("failed to connect() to app server: {0:?}")]
    ConnectToAppServer(Errno),
    #[error("unexpected response status {0}")]
    UnexpectedStatus(u16),
    #[error("response body started before headers")]
    DataBeforeHeaders,
    #[error("server stopped reading the request with code {0}")]
    SendStopped(u64),
    #[error("message decode error: {0}")]
    MessageDecode(#[from] MessageDecodeError),
    #[error("MessageWriter error: {0}")]
    MessageWriter(#[from] MessageWriterError),

    // h3 protocol errors
    #[error("failed to start QUIC connection: {0}")]
    Connect(#[from] quinn_proto::ConnectError),
    #[error("connection is closed: idle timeout")]
    IdleTimeout,
    #[error("connection is lost: {0}")]
    ConnectionLost(quinn_proto::ConnectionError),
    #[error("peer allows no more streams")]
    NoStreamsAvailable,
    #[error("request has already been sent")]
    RequestAlreadySent,
    #[error("request hasn't been sent yet")]
    NoRequest,
    #[error("failed to read stream: {0}")]
    Readable(ReadableError),
    #[error("failed to write stream: {0}")]
    Write(WriteError),
    #[error("HTTP/3 error: {0}")]
    H3(#[from] h3wire::ConnectionError),
    #[error("HTTP/3 usage error: {0}")]
    H3Usage(#[from] h3wire::UsageError),
    #[error("HTTP/3 connection is closed with code {0}")]
    Closed(u64),
    #[error("response has no :status")]
    MissingStatus,
    #[error("response has finished")]
    ResponseFinished,
    #[error("response has been reset with code {0}")]
    ResponseReset(u64),
    #[error("received GOAWAY")]
    GoAway,
}
