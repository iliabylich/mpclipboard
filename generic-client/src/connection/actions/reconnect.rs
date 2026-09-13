use crate::{
    config::Config,
    connection::{actions::connect, maybe_tls_stream::MaybeTlsStream},
};
use mpclipboard_shared::{
    Completion::{self, *},
    error,
};
use std::os::fd::OwnedFd;

pub fn reconnect(
    config: &Config,
) -> Completion<(OwnedFd, MaybeTlsStream), (OwnedFd, MaybeTlsStream)> {
    let addr = match config.url.resolve() {
        Ok(addr) => addr,
        Err(err) => {
            error!("failed to get IP address of the url: {err:?}");
            return Failed;
        }
    };

    let stream = match MaybeTlsStream::new(&config.url) {
        Ok(stream) => stream,
        Err(err) => {
            error!("failed to create MaybeTlsStream: {err:?}");
            return Failed;
        }
    };

    match connect(addr) {
        Done(fd) => Done((fd, stream)),
        Failed => Failed,
        Pending(fd) => Pending((fd, stream)),
    }
}
