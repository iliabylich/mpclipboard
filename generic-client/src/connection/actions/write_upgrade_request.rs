use crate::connection::maybe_tls_stream::MaybeTlsStream;
use anyhow::anyhow;
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Completion<(), anyhow::Error, ()> {
    let buf = writer.remainder();

    let len = match stream.write_bytes(&fd, buf) {
        Done(len) => len,
        Failed(err) => return Failed(err.into()),
        Pending(()) => return Pending(()),
    };

    writer.written(len).map_err(|err| anyhow!(err))
}
