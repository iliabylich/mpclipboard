use crate::{
    config::Config,
    connection::{
        actions::{ConnectResult, connect},
        maybe_tls_stream::MaybeTlsStream,
    },
};
use mpclipboard_shared::error;
use std::os::fd::OwnedFd;

pub fn reconnect(config: &Config) -> ReconnectResult {
    let addr = match config.url.resolve() {
        Ok(addr) => addr,
        Err(err) => {
            error!("failed to get IP address of the url: {err:?}");
            return ReconnectResult::Failed;
        }
    };

    let stream = match MaybeTlsStream::new(&config.url) {
        Ok(stream) => stream,
        Err(err) => {
            error!("failed to create MaybeTlsStream: {err:?}");
            return ReconnectResult::Failed;
        }
    };

    let fd = match connect(addr) {
        ConnectResult::Connected(fd) => fd,
        ConnectResult::StillPending(fd) => return ReconnectResult::Connecting { fd, stream },
        ConnectResult::Failed => return ReconnectResult::Failed,
    };

    if stream.is_tls() {
        ReconnectResult::ConnectedNeedsTlsHandshake { fd, stream }
    } else {
        ReconnectResult::ConnectedReadyStartHandshake { fd, stream }
    }
}

pub enum ReconnectResult {
    Failed,
    Connecting { fd: OwnedFd, stream: MaybeTlsStream },
    ConnectedNeedsTlsHandshake { fd: OwnedFd, stream: MaybeTlsStream },
    ConnectedReadyStartHandshake { fd: OwnedFd, stream: MaybeTlsStream },
}
