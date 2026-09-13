use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{Completion, Message, MessageReader};
use std::os::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<Message, ()> {
    let mut buf = [0; Message::BYTESIZE];
    stream
        .read_bytes(fd, &mut buf)
        .and_then(|len| reader.received(buf, len))
}
