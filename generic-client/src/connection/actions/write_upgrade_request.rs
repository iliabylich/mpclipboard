use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{UpgradeRequestWriter, UpgradeRequestWriterResult, error};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> WriteUpgradeRequestResult {
    let len = match stream.write_bytes(&fd, writer.remainder()) {
        Ok(Some(len)) => len,
        Ok(None) => return WriteUpgradeRequestResult::Pending,
        Err(err) => {
            error!("write() failed: {err:?}");
            return WriteUpgradeRequestResult::Error;
        }
    };

    match writer.written(len) {
        UpgradeRequestWriterResult::Done => WriteUpgradeRequestResult::Done,
        UpgradeRequestWriterResult::Pending => WriteUpgradeRequestResult::Pending,
        UpgradeRequestWriterResult::Error => {
            error!("write() failed");
            WriteUpgradeRequestResult::Error
        }
    }
}

pub enum WriteUpgradeRequestResult {
    Done,
    Pending,
    Error,
}
