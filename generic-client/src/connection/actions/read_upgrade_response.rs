use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{
    Message, MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, error, prelude::*, trace,
};
use std::os::fd::AsFd;

pub fn read_upgrade_response(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Completion<MessageReader, ()> {
    let mut buf = [0; UpgradeResponseReader::BUFFER_SIZE];
    let len = match stream.read_bytes(&fd, &mut buf) {
        Done(len) => len,
        Pending(()) => {
            trace!("handshake response still pending: {:?}", reader);
            return Pending(());
        }
        Failed => {
            error!("failed to read() handshake response");
            return Failed;
        }
    };

    let (leftover, leftover_len) = match reader.received(buf, len) {
        Done((leftover, leftover_len)) => {
            trace!("Handshake response matches");
            (leftover, leftover_len)
        }
        Pending(()) => {
            trace!("handshake response still pending: {:?}", reader);
            return Pending(());
        }
        Failed => {
            error!("failed to read() handshake response");
            return Failed;
        }
    };
    if let Err(err) = enable_tcp_keep_alive(&fd) {
        error!("{err:?}");
        return Failed;
    }

    let mut buf = [0; Message::BYTESIZE];
    const {
        assert!(UpgradeResponseReader::BUFFER_SIZE < Message::BYTESIZE);
    }
    buf[..UpgradeResponseReader::BUFFER_SIZE].copy_from_slice(&leftover);
    let reader = MessageReader::new(buf, leftover_len);
    Done(reader)
}
