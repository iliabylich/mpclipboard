use crate::connection::maybe_tls_stream::MaybeTlsStream;
use mpclipboard_shared::{
    Completion::{self, *},
    UpgradeRequestWriter, error,
};
use std::os::fd::AsFd;

pub fn write_upgrade_request(
    fd: impl AsFd,
    stream: &mut MaybeTlsStream,
    writer: &mut UpgradeRequestWriter,
) -> Completion<(), ()> {
    let len = match stream.write_bytes(&fd, writer.remainder()) {
        Ok(Some(len)) => len,
        Ok(None) => return Pending(()),
        Err(err) => {
            error!("write() failed: {err:?}");
            return Failed;
        }
    };

    match writer.written(len) {
        Done(()) => Done(()),
        Pending(()) => Pending(()),
        Failed => Failed,
    }
}
