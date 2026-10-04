use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{Message, MessageReader, prelude::*};
use rustix::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Result<Completion<Message, ()>, ConnectionError> {
    let mut message = None;

    loop {
        let Done(buf) = stream
            .read_bytes(fd)
            .map_err(ConnectionError::FailedToRead)?
        else {
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
