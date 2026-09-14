use crate::{
    config::Config,
    connection::{actions::connect, maybe_tls_stream::MaybeTlsStream},
};
use mpclipboard_shared::prelude::*;
use std::os::fd::OwnedFd;

pub fn reconnect(
    config: &Config,
) -> Completion<(OwnedFd, MaybeTlsStream), anyhow::Error, (OwnedFd, MaybeTlsStream)> {
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

    match connect(addr) {
        Done(fd) => Done((fd, stream)),
        Failed(err) => Failed(err),
        Pending(fd) => Pending((fd, stream)),
    }
}
