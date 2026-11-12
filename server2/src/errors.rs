use crate::handshake::ParseError;
use core::{ffi::CStr, net::AddrParseError, str::Utf8Error, time::Duration};
use externally_driven_async_rt::io::WriteAllError;
use mpclipboard_shared2::{FixedStrError, ID, MessageReaderError};
use rustix::io::Errno;

#[derive(Debug, PartialEq, Eq)]
pub enum ServerError {
    NonUtf8EnvVar { name: &'static CStr, err: Utf8Error },
    InvalidRustLog(&'static str),
    MissingBindAddress,
    InvalidBindAddress(AddrParseError),
    MissingToken,
    InvalidToken(FixedStrError),
    Bind { address: &'static str, err: Errno },
    Selector(Errno),
    EventLoopStart(externally_driven_async_rt::Error),
    Poll(Errno),
    EventLoop(externally_driven_async_rt::Error),
}

impl core::fmt::Display for ServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonUtf8EnvVar { name, err } => write!(
                f,
                "env var {} must be valid utf-8: {err}",
                name.to_str().unwrap_or("<non-utf8 name>")
            ),
            Self::InvalidRustLog(value) => write!(
                f,
                "invalid RUST_LOG=\"{value}\": expected one of off, error, warn, info, debug, trace"
            ),
            Self::MissingBindAddress => f.write_str("MPCLIPBOARD_BIND_ADDRESS is missing"),
            Self::InvalidBindAddress(err) => {
                write!(
                    f,
                    "MPCLIPBOARD_BIND_ADDRESS must be an ip:port address: {err}"
                )
            }
            Self::MissingToken => f.write_str("MPCLIPBOARD_TOKEN is missing"),
            Self::InvalidToken(err) => write!(f, "MPCLIPBOARD_TOKEN is malformed: {err}"),
            Self::Bind { address, err } => write!(f, "failed to bind {address}: {err}"),
            Self::Selector(err) => write!(f, "failed to create a selector: {err}"),
            Self::EventLoopStart(err) => write!(f, "failed to start the event loop: {err}"),
            Self::Poll(err) => write!(f, "poll() failed: {err}"),
            Self::EventLoop(err) => write!(f, "{err}"),
        }
    }
}

impl From<AddrParseError> for ServerError {
    fn from(err: AddrParseError) -> Self {
        Self::InvalidBindAddress(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientError {
    HandshakeTimeout(Duration),
    HandshakeRead(Errno),
    Handshake(ParseError),
    InvalidToken {
        id: ID,
    },
    BytesAfterHandshake {
        id: ID,
    },
    Read {
        id: ID,
        err: MessageReaderError<Errno>,
    },
    Write {
        id: ID,
        err: WriteAllError<Errno>,
    },
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HandshakeTimeout(timeout) => {
                write!(f, "no handshake within {}s", timeout.as_secs())
            }
            Self::HandshakeRead(err) => write!(f, "read() failed during handshake: {err}"),
            Self::Handshake(err) => write!(f, "{err}"),
            Self::InvalidToken { id } => write!(f, "[{id}] invalid token"),
            Self::BytesAfterHandshake { id } => {
                write!(f, "[{id}] sent bytes before receiving 101")
            }
            Self::Read { id, err } => write!(f, "[{id}] {err}"),
            Self::Write { id, err } => write!(f, "[{id}] {err}"),
        }
    }
}

impl From<ParseError> for ClientError {
    fn from(err: ParseError) -> Self {
        Self::Handshake(err)
    }
}
