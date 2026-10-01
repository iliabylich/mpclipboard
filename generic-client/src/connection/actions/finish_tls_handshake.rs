use crate::{config::Config, connection::maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::AsFd;

pub fn finish_tls_handshake(
    stream: &mut MaybeTlsStream,
    fd: &impl AsFd,
    config: &Config,
) -> Completion<UpgradeRequestWriter, anyhow::Error, ()> {
    match stream.finish_tls_handshake(fd) {
        Done(()) => {}
        Failed(err) => return Failed(err.into()),
        Pending(()) => return Pending(()),
    }

    Done(UpgradeRequestWriter::new(config.update_request()))
}
