mod connected;
mod connecting;
mod disconnected;
mod handshaking;
mod ready;

use std::os::fd::AsRawFd as _;

pub(crate) use connected::Connected;
pub(crate) use connecting::Connecting;
pub(crate) use disconnected::Disconnected;
pub(crate) use handshaking::Handshaking;
pub(crate) use ready::Ready;

#[derive(Default)]
pub(crate) enum State {
    Connected(Connected),
    Connecting(Connecting),
    Handshaking(Handshaking),
    Ready(Ready),
    Disconnected(Disconnected),

    #[default]
    Taken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateTag {
    Connected,
    Connecting,
    Handshaking,
    Ready,
    Disconnected,
    Taken,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected(_) => write!(f, "Connected"),
            Self::Connecting(_) => write!(f, "Connecting"),
            Self::Handshaking(_) => write!(f, "Handshaking"),
            Self::Ready(_) => write!(f, "Ready"),
            Self::Disconnected(_) => write!(f, "Disconnected"),
            Self::Taken => write!(f, "Taken"),
        }
    }
}

impl State {
    pub(crate) fn tag(&self) -> StateTag {
        match self {
            State::Connected(_) => StateTag::Connected,
            State::Connecting(_) => StateTag::Connecting,
            State::Handshaking(_) => StateTag::Handshaking,
            State::Ready(_) => StateTag::Ready,
            State::Disconnected(_) => StateTag::Disconnected,
            State::Taken => StateTag::Taken,
        }
    }

    // readable -> writable -> fd
    pub(crate) fn interests(
        &self,
        write_blocked: bool,
        has_pending_message: bool,
    ) -> Option<(bool, bool, i32)> {
        match self {
            State::Connecting(Connecting(fd)) => Some((false, true, fd.as_raw_fd())),
            State::Connected(Connected(fd)) => Some((true, true, fd.as_raw_fd())),
            State::Handshaking(Handshaking(mid_handshake)) => {
                Some((true, true, mid_handshake.as_raw_fd()))
            }
            State::Ready(Ready(ws)) => {
                Some((true, write_blocked || has_pending_message, ws.as_raw_fd()))
            }
            State::Disconnected(_) => None,
            State::Taken => {
                unreachable!("bug: trying to update Taken connection");
            }
        }
    }
}
