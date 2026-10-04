use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::prelude::*;
use rustix::fd::AsFd;

pub fn finish_tls_handshake(
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<(), ()>, ConnectionError> {
    stream
        .finish_tls_handshake(fd)
        .map_err(ConnectionError::FailedToFinishTlsHandshake)
}
