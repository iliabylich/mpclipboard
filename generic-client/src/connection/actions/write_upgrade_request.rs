use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{Completion, UpgradeRequestWriter};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Completion<(), anyhow::Error, ()> {
    stream
        .write_bytes(&fd, writer.remainder())
        .and_then(|len| writer.written(len))
}
