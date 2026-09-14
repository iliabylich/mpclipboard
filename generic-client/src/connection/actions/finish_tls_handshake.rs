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
        Failed(err) => return Failed(err),
        Pending(()) => return Pending(()),
    }

    let writer = match UpgradeRequestWriter::new(config.update_request()) {
        Ok(writer) => writer,
        Err(err) => return Failed(err),
    };

    Done(writer)
}
