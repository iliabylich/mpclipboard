use crate::connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError};
use mpclipboard_shared::{Message, MessageError, MessageReader, prelude::*};
use rustix::fd::AsFd;

pub(crate) fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<Message, ()>, ReadMessageError> {
    let mut message = None;

    loop {
        let Done(buf) = stream.read_bytes(fd)? else {
            break;
        };

        if let Done(m) = reader.received(buf)? {
            message = Some(m);
        }
    }

    if let Some(message) = message {
        Ok(Done(message))
    } else {
        Ok(Pending(()))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadMessageError {
    #[error("failed to read: {0}")]
    Read(#[from] MaybeTlsStreamError),
    #[error("message error: {0}")]
    Message(#[from] MessageError),
}
