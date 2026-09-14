use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{MessageWriter, prelude::*};
use std::os::fd::AsFd;

pub fn write_message(
    writer: &mut MessageWriter,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<(), anyhow::Error, ()> {
    if writer.is_empty()
        && let Err(err) = stream.flush(fd)
    {
        return Failed(err.context("failed to flush TLS data"));
    }

    let Some(buf) = writer.remainder() else {
        return Done(());
    };

    let len = match stream.write_bytes(fd, buf) {
        Done(len) => len,
        Failed(err) => return Failed(err),
        Pending(()) => return Pending(()),
    };

    match writer.written(len) {
        Ok(()) => Done(()),
        Err(err) => Failed(err),
    }
}
