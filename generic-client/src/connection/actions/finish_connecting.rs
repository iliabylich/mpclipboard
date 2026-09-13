use mpclipboard_shared::error;
use std::os::fd::AsFd;

pub fn finish_connecting(fd: impl AsFd) -> FinishConnectingResult {
    match rustix::net::sockopt::socket_error(fd) {
        Ok(Ok(())) => FinishConnectingResult::Connected,
        Ok(Err(err)) | Err(err) => {
            error!("socket_error returned error: {err:?}");
            FinishConnectingResult::FailedToConnect
        }
    }
}

pub enum FinishConnectingResult {
    Connected,
    FailedToConnect,
}
