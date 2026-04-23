mod connected;
mod connecting;
mod disconnected;
mod handshaking;
mod ready;

pub(crate) use connected::Connected;
pub(crate) use connecting::Connecting;
pub(crate) use disconnected::Disconnected;
pub(crate) use handshaking::Handshaking;
pub(crate) use ready::Ready;

pub(crate) enum State {
    Connected(Connected),
    Connecting(Connecting),
    Handshaking(Handshaking),
    Ready(Ready),
    Disconnected(Disconnected),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateTag {
    Connected,
    Connecting,
    Handshaking,
    Ready,
    Disconnected,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected(_) => write!(f, "Connected"),
            Self::Connecting(_) => write!(f, "Connecting"),
            Self::Handshaking(_) => write!(f, "Handshaking"),
            Self::Ready(_) => write!(f, "Ready"),
            Self::Disconnected(_) => write!(f, "Disconnected"),
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
        }
    }
}
