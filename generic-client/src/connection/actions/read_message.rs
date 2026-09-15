use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{Message, MessageReader, prelude::*};
use std::os::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<Message, anyhow::Error, ()> {
    let mut buf = [0; Message::BYTESIZE];
    let mut message = None;

    loop {
        let len = match stream.read_bytes(fd, &mut buf) {
            Done(len) => len,
            Failed(err) => return Failed(err),
            Pending(()) => break,
        };

        match reader.received(buf, len) {
            Done(m) => message = Some(m),
            Failed(err) => return Failed(err),
            Pending(()) => {}
        }
    }

    if let Some(message) = message {
        Done(message)
    } else {
        Pending(())
    }
}
