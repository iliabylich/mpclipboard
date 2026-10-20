use crate::connection::{actions::ConnectError, http3_connection::Http3Error};
use mpclipboard_shared::{MessageError, MessageWriterError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Http3StateError {
    #[error("failed to open UDP socket: {0}")]
    Socket(#[from] ConnectError),
    #[error("HTTP/3 connection error: {0}")]
    Connection(#[from] Http3Error),
    #[error("max UDP payload size doesn't fit into usize: {0}")]
    BufferSize(#[from] core::num::TryFromIntError),
    #[error("unexpected response status {0}")]
    UnexpectedStatus(u16),
    #[error("response body started before headers")]
    DataBeforeHeaders,
    #[error("server stopped reading the request with code {0}")]
    SendStopped(u64),
    #[error("message error: {0}")]
    Message(#[from] MessageError),
    #[error("MessageWriter error: {0}")]
    Writer(#[from] MessageWriterError),
}
