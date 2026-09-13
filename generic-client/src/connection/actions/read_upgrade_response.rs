use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{
    Message, MessageReader, UpgradeResponseReader, UpgradeResponseReaderResult,
    enable_tcp_keep_alive, error, trace,
};
use std::os::fd::AsFd;

pub fn read_upgrade_response(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    reader: &mut UpgradeResponseReader,
) -> ReadUpgradeResponseResult {
    let mut buf = [0; UpgradeResponseReader::BUFFER_SIZE];
    let len = match stream.read_bytes(&fd, &mut buf) {
        Ok(Some(len)) => len,
        Ok(None) => {
            trace!("handshake response still pending: {:?}", reader);
            return ReadUpgradeResponseResult::Pending;
        }
        Err(err) => {
            error!("failed to read() handshake response: {err:?}");
            return ReadUpgradeResponseResult::Error;
        }
    };

    let (leftover, leftover_len) = match reader.received(buf, len) {
        UpgradeResponseReaderResult::Done {
            leftover,
            leftover_len,
        } => {
            trace!("Handshake response matches");
            (leftover, leftover_len)
        }
        UpgradeResponseReaderResult::Pending => {
            trace!("handshake response still pending: {:?}", reader);
            return ReadUpgradeResponseResult::Pending;
        }
        UpgradeResponseReaderResult::Error => {
            error!("failed to read() handshake response");
            return ReadUpgradeResponseResult::Error;
        }
    };
    if let Err(err) = enable_tcp_keep_alive(&fd) {
        error!("{err:?}");
        return ReadUpgradeResponseResult::Error;
    }

    let mut buf = [0; Message::BYTESIZE];
    const _: () = assert!(UpgradeResponseReader::BUFFER_SIZE < Message::BYTESIZE);
    buf[..UpgradeResponseReader::BUFFER_SIZE].copy_from_slice(&leftover);
    let reader = MessageReader::new(buf, leftover_len);
    ReadUpgradeResponseResult::Done { reader }
}

pub enum ReadUpgradeResponseResult {
    Done { reader: MessageReader },
    Pending,
    Error,
}
