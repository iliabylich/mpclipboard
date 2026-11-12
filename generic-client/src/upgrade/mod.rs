mod parse;
mod reader;
mod writer;

pub(crate) use reader::UpgradeReader;
pub(crate) use writer::UpgradeWriter;

use crate::{constants::UPGRADE_RESPONSE_BUFFER_SIZE, tls_stream::TlsError};
use externally_driven_async_rt::io::{ReadExactError, WriteAllError};
use mpclipboard_shared2::UPGRADE_PROTOCOL;

#[derive(Debug, thiserror::Error)]
pub(crate) enum UpgradeError {
    #[error(transparent)]
    Tls(#[from] TlsError),
    #[error("failed to write HTTP request: {0}")]
    Write(#[from] WriteAllError<TlsError>),
    #[error("failed to read HTTP response: {0}")]
    Read(#[from] ReadExactError<TlsError>),
    #[error("malformed HTTP response: {0}")]
    InvalidHttpResponse(#[from] httparse::Error),
    #[error("HTTP response exceeds {UPGRADE_RESPONSE_BUFFER_SIZE} bytes")]
    TooLarge,
    #[error("server rejected the token")]
    Unauthorized,
    #[error("unexpected HTTP status {0:?}, expected 101")]
    UnexpectedStatus(Option<u16>),
    #[error("response is not Connection: Upgrade")]
    NotUpgrade,
    #[error("response is not Upgrade: {UPGRADE_PROTOCOL}")]
    UnsupportedUpgrade,
}
