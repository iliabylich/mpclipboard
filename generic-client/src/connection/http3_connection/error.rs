use quinn_proto::{ReadableError, WriteError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Http3Error {
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
    #[error("length doesn't fit into u64: {0}")]
    Length(#[from] core::num::TryFromIntError),
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
