use crate::connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError};
use mpclipboard_shared::{UpgradeRequestWriter, UpgradeRequestWriterError, prelude::*};
use rustix::fd::AsFd;

pub(crate) fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Result<Completion<(), ()>, WriteUpgradeRequestError> {
    let Done(len) = stream.write_bytes(&fd, writer.remainder())? else {
        return Ok(Pending(()));
    };

    let written = writer.written(len)?;
    Ok(written)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WriteUpgradeRequestError {
    #[error("failed to write: {0}")]
    Write(#[from] MaybeTlsStreamError),
    #[error("UpgradeRequestWriter error: {0}")]
    Writer(#[from] UpgradeRequestWriterError),
}
