use crate::connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError};
use mpclipboard_shared::{MessageWriter, MessageWriterError, prelude::*};
use rustix::fd::AsFd;

pub fn write_message(
    writer: &mut MessageWriter,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<(), ()>, WriteMessageError> {
    if writer.is_empty() {
        stream.flush(fd).map_err(WriteMessageError::FlushTls)?;
    }

    let Some(buf) = writer.remainder() else {
        return Ok(Done(()));
    };

    let Done(len) = stream
        .write_bytes(fd, buf)
        .map_err(WriteMessageError::Write)?
    else {
        return Ok(Pending(()));
    };

    writer.written(len)?;
    Ok(Done(()))
}

#[derive(Debug, thiserror::Error)]
pub enum WriteMessageError {
    #[error("failed to flush TLS data: {0}")]
    FlushTls(MaybeTlsStreamError),
    #[error("failed to write: {0}")]
    Write(MaybeTlsStreamError),
    #[error("MessageWriter error: {0}")]
    Writer(#[from] MessageWriterError),
}
