use crate::{
    Connectivity,
    config::Config,
    connection::{error::ConnectionError, maybe_tls_stream::MaybeTlsStream},
};
use mpclipboard_shared::{Message, Wants, prelude::*};
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};

mod disconnected;
pub use disconnected::DisconnectedState;

mod dns;
use dns::DnsState;

mod tcp;
pub use tcp::TcpState;

const FREEZE_TIME_IN_SECS: u64 = 3;

#[derive(Debug)]
pub enum State {
    Disconnected(DisconnectedState),
    Dns(DnsState),
    Tcp {
        fd: OwnedFd,
        stream: MaybeTlsStream,
        state: TcpState,
    },
}

impl State {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Disconnected(_) => DisconnectedState::name(),
            Self::Dns(_) => DnsState::name(),
            Self::Tcp { state, .. } => state.name(),
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Disconnected(_) => DisconnectedState::connectivity(),
            Self::Dns(_) => DnsState::connectivity(),
            Self::Tcp { state, .. } => state.connectivity(),
        }
    }

    pub(crate) fn tick(&self, now: u64, config: &Config) -> Option<Self> {
        let is_stuck = match self {
            Self::Disconnected(state) => {
                return match state.tick(now, config) {
                    Ok(next) => next,
                    Err(err) => {
                        log::error!("failed to reconnect: {err}");
                        Some(Self::Disconnected(DisconnectedState::new(Some(now))))
                    }
                };
            }

            Self::Dns(state) => state.is_stuck(now),
            Self::Tcp { state, .. } => state.is_stuck(now),
        };

        if !is_stuck {
            return None;
        }

        log::error!("Stuck in {}, disconnecting...", self.name());
        Some(Self::Disconnected(DisconnectedState::new(Some(now))))
    }

    pub(crate) fn on_readable(
        &mut self,
        now: u64,
        config: &Config,
    ) -> Result<Completion<Readable, ()>, ConnectionError> {
        match self {
            Self::Disconnected(_) => unreachable!("can't read() in Disconnected state"),

            Self::Dns(state) => {
                let Done(next) = state.on_readable(now, config)? else {
                    return Ok(Pending(()));
                };
                Ok(Done(Readable::NextState(next)))
            }

            Self::Tcp { fd, stream, state } => {
                let Done(message) = state.on_readable(fd, stream, now, config)? else {
                    return Ok(Pending(()));
                };
                Ok(Done(Readable::Message(message)))
            }
        }
    }

    pub(crate) fn on_writable(&mut self, now: u64, config: &Config) -> Result<(), ConnectionError> {
        match self {
            Self::Disconnected(_) => unreachable!("can't write() in Disconnected state"),
            Self::Dns(state) => state.on_writable(now),
            Self::Tcp { fd, stream, state } => state.on_writable(fd, stream, now, config),
        }
    }

    pub(crate) fn wants(&mut self) -> Option<(BorrowedFd<'_>, Wants)> {
        match self {
            Self::Disconnected(_) => DisconnectedState::wants(),
            Self::Dns(state) => Some(state.wants()),
            Self::Tcp { fd, stream, state } => Some(((*fd).as_fd(), state.wants(stream))),
        }
    }
}

pub enum Readable {
    NextState(State),
    Message(Message),
}
