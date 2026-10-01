use crate::connection::maybe_tls_stream::MaybeTlsStream;
use anyhow::anyhow;
use mpclipboard_shared::{MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, prelude::*};
use std::os::fd::AsFd;

pub fn read_upgrade_response(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Completion<MessageReader, anyhow::Error, ()> {
    let buf = match stream.read_bytes(&fd) {
        Done(buf) => buf,
        Pending(()) => {
            log::trace!("handshake response still pending: {reader:?}");
            return Pending(());
        }
        Failed(err) => return Failed(err.into()),
    };

    let leftover = match reader.received(buf) {
        Done(leftover) => {
            log::trace!("Handshake response matches");
            leftover
        }
        Pending(()) => {
            log::trace!("handshake response still pending: {reader:?}");
            return Pending(());
        }
        Failed(err) => {
            return Failed(anyhow!(err));
        }
    };

    log::trace!("Configuring TCP keepalive");
    if let Err(err) = enable_tcp_keep_alive(&fd) {
        return Failed(anyhow!(err));
    }

    Done(MessageReader::new(leftover))
}
