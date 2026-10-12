use crate::connection::maybe_tls_stream::{MaybeTlsStream, MaybeTlsStreamError};
use mpclipboard_shared::{
    MessageReader, TcpKeepAliveError, UpgradeResponseReader, UpgradeResponseReaderError,
    enable_tcp_keep_alive, prelude::*,
};
use rustix::fd::AsFd;

pub fn read_upgrade_response(
    fd: &impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> Result<Completion<MessageReader, ()>, ReadUpgradeResponseError> {
    let leftover = loop {
        let Done(buf) = stream.read_bytes(fd)? else {
            log::trace!("handshake response still pending: {reader:?}");
            return Ok(Pending(()));
        };

        if let Done(leftover) = reader.received(buf)? {
            log::trace!("Handshake response matches");
            break leftover;
        }
    };

    log::trace!("Configuring TCP keepalive");
    enable_tcp_keep_alive(fd)?;

    Ok(Done(MessageReader::new(leftover)))
}

#[derive(Debug, thiserror::Error)]
pub enum ReadUpgradeResponseError {
    #[error("failed to read: {0}")]
    Read(#[from] MaybeTlsStreamError),
    #[error("UpgradeResponseReader error: {0}")]
    Reader(#[from] UpgradeResponseReaderError),
    #[error("TCP keepalive error: {0}")]
    TcpKeepAlive(#[from] TcpKeepAliveError),
}
