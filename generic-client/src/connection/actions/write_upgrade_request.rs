use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Completion<(), anyhow::Error, ()> {
    let buf = match writer.remainder() {
        Ok(buf) => buf,
        Err(err) => return Failed(err),
    };

    let len = match stream.write_bytes(&fd, buf) {
        Done(len) => len,
        Failed(err) => return Failed(err),
        Pending(()) => return Pending(()),
    };

    writer.written(len)
}
