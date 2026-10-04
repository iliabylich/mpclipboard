use crate::connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream};
use mpclipboard_shared::{MessageReader, UpgradeResponseReader, enable_tcp_keep_alive, prelude::*};
use rustix::fd::AsFd;

pub fn read_upgrade_response(
    fd: &impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Result<Completion<MessageReader, ()>, ConnectionError> {
    let leftover = loop {
        let Done(buf) = stream
            .read_bytes(fd)
            .map_err(ConnectionError::FailedToRead)?
        else {
            log::trace!("handshake response still pending: {reader:?}");
            return Ok(Pending(()));
        };

        if let Done(leftover) = reader
            .received(buf)
            .map_err(ConnectionError::UpgradeResponseReaderError)?
        {
            log::trace!("Handshake response matches");
            break leftover;
        }
    };

    log::trace!("Configuring TCP keepalive");
    enable_tcp_keep_alive(fd).map_err(ConnectionError::TcpKeepAliveError)?;

    Ok(Done(MessageReader::new(leftover)))
}
