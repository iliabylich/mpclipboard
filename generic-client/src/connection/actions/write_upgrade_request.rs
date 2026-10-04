use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use rustix::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Result<Completion<(), ()>, ConnectionError> {
    let Done(len) = stream
        .write_bytes(&fd, writer.remainder())
        .map_err(ConnectionError::FailedToWrite)?
    else {
        return Ok(Pending(()));
    };

    let written = writer.written(len)?;
    Ok(written)
}
