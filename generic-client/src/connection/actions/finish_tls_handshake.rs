use crate::connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError};
use mpclipboard_shared::prelude::*;
use rustix::fd::AsFd;

pub(crate) fn finish_tls_handshake(
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<(), ()>, FinishTlsHandshakeError> {
    let completion = stream.finish_tls_handshake(fd)?;
    Ok(completion)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FinishTlsHandshakeError {
    #[error("failed to finish TLS handshake: {0}")]
    Handshake(#[from] MaybeTlsStreamError),
}
