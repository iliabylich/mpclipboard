use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{MessageWriter, error};
use std::os::fd::AsFd;

pub fn write_message(
    writer: &mut MessageWriter,
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
) -> WriteMessageResult {
    if writer.is_empty()
        && let Err(err) = stream.flush(fd)
    {
        error!("failed to flush TLS data: {err:?}");
        return WriteMessageResult::Error;
    }

    let Some(buf) = writer.remainder() else {
        return WriteMessageResult::Ok;
    };
    match stream.write_bytes(fd, buf) {
        Ok(Some(len)) => {
            writer.written(len);
            WriteMessageResult::Ok
        }
        Ok(None) => WriteMessageResult::Ok,
        Err(err) => {
            error!("failed to write(): {err:?}");
            WriteMessageResult::Error
        }
    }
}

pub enum WriteMessageResult {
    Ok,
    Error,
}
