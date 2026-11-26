use crate::{
    constants::UPGRADE_TIMEOUT_IN_SECS,
    constants::vars::{BIND_ADDRESS, TOKEN},
    upgrade::{ParseError, UpgradeWriterError},
};
use core::net::AddrParseError;
use externally_driven_async_rt::{EventLoopError, io::FrameWriterError, net::NetError};
use mpclipboard_shared::{FixedStrError, ID, LoggerError, MessageReaderError, env::EnvVarError};
use rustix::io::Errno;

#[derive(Debug, PartialEq, Eq)]
pub enum ServerError {
    NonUtf8EnvVar(EnvVarError),
    Logger(LoggerError),
    MissingBindAddress,
    InvalidBindAddress(AddrParseError),
    MissingToken,
    InvalidToken(FixedStrError),
    Selector(Errno),
    EventLoopStart(EventLoopError),
    Poll(Errno),
    EventLoop(EventLoopError),
}

impl core::fmt::Display for ServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonUtf8EnvVar(err) => write!(f, "{err}"),
            Self::Logger(err) => write!(f, "{err}"),
            Self::MissingBindAddress => write!(f, "{BIND_ADDRESS:?} is missing"),
            Self::InvalidBindAddress(err) => write!(f, "malformed {BIND_ADDRESS:?}: {err}"),
            Self::MissingToken => write!(f, "{TOKEN:?} is missing"),
            Self::InvalidToken(err) => write!(f, "{TOKEN:?} is malformed: {err}"),
            Self::Selector(err) => write!(f, "failed to create a selector: {err}"),
            Self::EventLoopStart(err) => write!(f, "failed to start the event loop: {err}"),
            Self::Poll(err) => write!(f, "poll() failed: {err}"),
            Self::EventLoop(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientError {
    UpgradeTimeout,
    UpgradeRead(NetError),
    DisconnectedDuringUpgrade,
    Upgrade(ParseError),
    InvalidToken(ID),
    BytesAfterUpgrade(ID),
    UpgradeWrite {
        id: ID,
        err: UpgradeWriterError,
    },
    Read {
        id: ID,
        err: MessageReaderError<NetError>,
    },
    Write {
        id: ID,
        err: FrameWriterError<NetError>,
    },
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UpgradeTimeout => write!(f, "no upgrade within {UPGRADE_TIMEOUT_IN_SECS}s"),
            Self::UpgradeRead(err) => write!(f, "read() failed during upgrade: {err}"),
            Self::DisconnectedDuringUpgrade => f.write_str("disconnected during upgrade"),
            Self::Upgrade(err) => write!(f, "{err}"),
            Self::InvalidToken(id) => write!(f, "[{id}] invalid token"),
            Self::BytesAfterUpgrade(id) => write!(f, "[{id}] received data after upgrade"),
            Self::UpgradeWrite { id, err } => write!(f, "[{id}] failed to write 101: {err}"),
            Self::Read { id, err } => write!(f, "[{id}] {err}"),
            Self::Write { id, err } => write!(f, "[{id}] {err}"),
        }
    }
}
