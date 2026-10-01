use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Completion<(), ConnectionError, ()> {
    let len = match stream.write_bytes(&fd, writer.remainder()) {
        Done(len) => len,
        Failed(err) => return Failed(ConnectionError::FailedToWrite(err)),
        Pending(()) => return Pending(()),
    };

    writer
        .written(len)
        .map_err(ConnectionError::UpgradeRequestWriterError)
}
