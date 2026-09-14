use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{MessageWriter, error, prelude::*};
use std::os::fd::AsFd;

pub fn write_message(
    writer: &mut MessageWriter,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> Completion<(), ()> {
    if writer.is_empty()
        && let Err(err) = stream.flush(fd)
    {
        error!("failed to flush TLS data: {err:?}");
        return Failed;
    }

    let Some(buf) = writer.remainder() else {
        return Done(());
    };

    stream.write_bytes(fd, buf).and_then(|len| {
        writer.written(len);
        Done(())
    })
}
