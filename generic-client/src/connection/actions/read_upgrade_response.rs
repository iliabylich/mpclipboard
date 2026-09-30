use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, prelude::*};
use std::os::fd::AsFd;

pub fn read_upgrade_response(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Completion<MessageReader, anyhow::Error, ()> {
    let mut buf = [0; UpgradeResponseReader::BUFFER_SIZE];
    let len = match stream.read_bytes(&fd, &mut buf) {
        Done(len) => len,
        Pending(()) => {
            log::trace!("handshake response still pending: {reader:?}");
            return Pending(());
        }
        Failed(err) => return Failed(err),
    };

    let leftover = match reader.received(buf, len) {
        Done(leftover) => {
            log::trace!("Handshake response matches");
            leftover
        }
        Pending(()) => {
            log::trace!("handshake response still pending: {reader:?}");
            return Pending(());
        }
        Failed(err) => {
            return Failed(err);
        }
    };

    log::trace!("Configuring TCP keepalive");
    if let Err(err) = enable_tcp_keep_alive(&fd) {
        return Failed(err);
    }

    Done(MessageReader::new(leftover))
}
