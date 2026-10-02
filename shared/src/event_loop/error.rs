use rustix::io::Errno;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventLoopError {
    #[error("failed to create event loop: {0:?}")]
    Create(Errno),
    #[error("failed to create timer: {0:?}")]
    CreateTimer(Errno),
    #[error("failed to register timer: {0:?}")]
    AddTimer(Errno),
    #[error("failed to read timer: {0:?}")]
    ReadTimer(Errno),
    #[error("failed to register connection fd: {0:?}")]
    Sync(Errno),
    #[error("failed to wait for events: {0:?}")]
    Wait(Errno),
}
