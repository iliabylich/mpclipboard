use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{
    Completion::{self, *},
    Message, MessageReader, error,
};
use std::os::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<Message, ()> {
    let mut buf = [0; Message::BYTESIZE];
    let len = match stream.read_bytes(fd, &mut buf) {
        Ok(Some(len)) => len,
        Ok(None) => return Pending(()),
        Err(err) => {
            error!("failed to read(): {err:?}");
            return Failed;
        }
    };

    match reader.received(buf, len) {
        Done(message) => Done(message),
        Failed => Failed,
        Pending(()) => Pending(()),
    }
}
