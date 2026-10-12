use rustix::{fd::AsFd, io::Errno};

pub fn finish_connecting(fd: impl AsFd) -> Result<(), FinishConnectingError> {
    rustix::net::sockopt::socket_error(fd)??;
    Ok(())
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum FinishConnectingError {
    #[error("failed to finish connecting: {0:?}")]
    Connect(#[from] Errno),
}
