use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{
    Message, MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, prelude::*,
};
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

    let (leftover, leftover_len) = match reader.received(buf, len) {
        Done((leftover, leftover_len)) => {
            log::trace!("Handshake response matches");
            (leftover, leftover_len)
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

    let mut buf = [0; Message::BYTESIZE];
    const {
        assert!(UpgradeResponseReader::BUFFER_SIZE < Message::BYTESIZE);
    }
    buf[..UpgradeResponseReader::BUFFER_SIZE].copy_from_slice(&leftover);
    let reader = MessageReader::new(buf, leftover_len);
    Done(reader)
}
