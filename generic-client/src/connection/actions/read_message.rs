use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{Message, MessageReader, error};
use std::os::fd::AsFd;

pub fn read_message(
    reader: &mut MessageReader,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> ReadMessageResult {
    let mut buf = [0; Message::BYTESIZE];
    let len = match stream.read_bytes(fd, &mut buf) {
        Ok(Some(len)) => len,
        Ok(None) => return ReadMessageResult::Pending,
        Err(err) => {
            error!("failed to read(): {err:?}");
            return ReadMessageResult::Error;
        }
    };

    match reader.received(buf, len) {
        Ok(Some(message)) => ReadMessageResult::Done { message },
        Ok(None) => ReadMessageResult::Pending,
        Err(err) => {
            error!("failed to decode message: {err:?}");
            ReadMessageResult::Error
        }
    }
}

pub enum ReadMessageResult {
    Done { message: Message },
    Pending,
    Error,
}
