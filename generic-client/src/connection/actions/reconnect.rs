use crate::{
    config::Config,
    connection::{actions::connect, maybe_tls_stream::MaybeTlsStream},
};
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::OwnedFd;

pub fn reconnect(
    config: &Config,
) -> Completion<
    (OwnedFd, MaybeTlsStream, UpgradeRequestWriter),
    anyhow::Error,
    (OwnedFd, MaybeTlsStream),
> {
    let addr = match config.url.resolve() {
        Ok(addr) => addr,
        Err(err) => {
            return Failed(err.context("failed to resolve URL"));
        }
    };

    let stream = match MaybeTlsStream::new(&config.url) {
        Ok(stream) => stream,
        Err(err) => {
            return Failed(err.context("failed to create MaybeTlsStream"));
        }
    };

    let upgrade_request_writer = match UpgradeRequestWriter::new(config.update_request()) {
        Ok(writer) => writer,
        Err(err) => return Failed(err.context("failed to construct UpgradeRequestWriter")),
    };

    match connect(addr) {
        Done(fd) => Done((fd, stream, upgrade_request_writer)),
        Failed(err) => Failed(err),
        Pending(fd) => Pending((fd, stream)),
    }
}
