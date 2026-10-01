use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{Message, MessageReader, prelude::*};
use std::os::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<Message, ConnectionError, ()> {
    let mut message = None;

    loop {
        let buf = match stream.read_bytes(fd) {
            Done(buf) => buf,
            Failed(err) => return Failed(ConnectionError::FailedToRead(err)),
            Pending(()) => break,
        };

        match reader.received(buf) {
            Done(m) => message = Some(m),
            Failed(err) => return Failed(ConnectionError::MessageError(err)),
            Pending(()) => {}
        }
    }

    if let Some(message) = message {
        Done(message)
    } else {
        Pending(())
    }
}
