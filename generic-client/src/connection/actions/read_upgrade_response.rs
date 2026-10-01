use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, prelude::*};
use std::os::fd::AsFd;

pub fn read_upgrade_response(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Completion<MessageReader, ConnectionError, ()> {
    let buf = match stream.read_bytes(&fd) {
        Done(buf) => buf,
        Pending(()) => {
            log::trace!("handshake response still pending: {reader:?}");
            return Pending(());
        }
        Failed(err) => return Failed(ConnectionError::FailedToRead(err)),
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
            return Failed(ConnectionError::UpgradeResponseReaderError(err));
        }
    };

    log::trace!("Configuring TCP keepalive");
    if let Err(err) = enable_tcp_keep_alive(&fd) {
        return Failed(ConnectionError::TcpKeepAliveError(err));
    }

    Done(MessageReader::new(leftover))
}
