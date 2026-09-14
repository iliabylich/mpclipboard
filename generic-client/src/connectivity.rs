use crate::connection::{Connection, State};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub enum Connectivity {
    Connecting,
    Connected,
    Disconnected,
}

impl Connectivity {
    pub(crate) const fn new(conn: &Connection) -> Self {
        match conn {
            Connection::Disconnected { .. } => Self::Disconnected,
            Connection::State {
                state: State::Connected { .. },
                ..
            } => Self::Connected,
            Connection::State { .. } => Self::Connecting,
        }
    }
}
