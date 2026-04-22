use crate::state::State;

/// Connectivity of the MPClipboard, emitted in `on_connectivity_changed`
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Connectivity {
    /// Connecting to remote server, performing handshake/auth
    Connecting,
    /// Connected, ready to talk
    Connected,
    /// Disconnected
    Disconnected,
}

impl From<&State> for Connectivity {
    fn from(state: &State) -> Self {
        match state {
            State::Connected(_) => Self::Connecting,
            State::Connecting(_) => Self::Connecting,
            State::Handshaking(_) => Self::Connecting,
            State::Ready(_) => Self::Connected,
            State::Disconnected(_) => Self::Disconnected,
            State::Taken => unreachable!(),
        }
    }
}
