use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{MessageWriter, prelude::*};
use rustix::fd::AsFd;

pub fn write_message(
    writer: &mut MessageWriter,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<(), ()>, ConnectionError> {
    if writer.is_empty() {
        stream
            .flush(fd)
            .map_err(ConnectionError::FailedToFlushTls)?;
    }

    let Some(buf) = writer.remainder() else {
        return Ok(Done(()));
    };

    let Done(len) = stream
        .write_bytes(fd, buf)
        .map_err(ConnectionError::FailedToWrite)?
    else {
        return Ok(Pending(()));
    };

    writer.written(len)?;
    Ok(Done(()))
}
