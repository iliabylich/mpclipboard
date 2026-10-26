use crate::{
    Connectivity,
    config::Config,
    connection::{error::ConnectionError, quic_config::QuicConfig},
    event_loop::EventLoop,
};
use mpclipboard_shared::{Message, Now, prelude::*};
use std::time::Instant;

mod disconnected;
pub(crate) use disconnected::Disconnected;

mod dns;
use dns::Dns;

mod http3;
pub(crate) use http3::Http3;

pub(crate) const FREEZE_TIME_IN_SECS: u64 = 3;
pub(crate) const HEARTBEAT_INTERVAL_IN_SECS: u64 = 30;

#[derive(Debug)]
pub(crate) enum State {
    Disconnected(Disconnected),
    Dns(Dns),
    Http3(Http3),
    Stopped,
}

impl State {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Disconnected(_) => Disconnected::NAME,
            Self::Dns(_) => Dns::NAME,
            Self::Http3(state) => state.name(),
            Self::Stopped => "Stopped",
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Disconnected(_) | Self::Stopped => Disconnected::CONNECTIVITY,
            Self::Dns(_) => Dns::CONNECTIVITY,
            Self::Http3(state) => state.connectivity(),
        }
    }

    pub(crate) fn tick(
        &mut self,
        now: Now,
        config: &Config,
        quic_config: &QuicConfig,
        event_loop: &EventLoop,
    ) -> Option<Self> {
        let is_stuck = match self {
            Self::Disconnected(state) => {
                return match state.tick(now, config, quic_config, event_loop) {
                    Ok(next) => next,
                    Err(err) => {
                        log::error!("failed to reconnect: {err}");
                        Some(Self::Disconnected(Disconnected::new(Some(now.as_past()))))
                    }
                };
            }

            Self::Dns(state) => state.is_stuck(now),
            Self::Http3(state) => {
                state.on_tick(now);
                state.is_stuck(now)
            }
            Self::Stopped => return None,
        };

        if !is_stuck {
            return None;
        }

        log::error!("Stuck in {}, disconnecting...", self.name());
        Some(Self::Disconnected(Disconnected::new(Some(now.as_past()))))
    }

    pub(crate) fn on_readable(
        &mut self,
        now: Now,
        config: &Config,
        quic_config: &QuicConfig,
        event_loop: &EventLoop,
    ) -> Result<Completion<Readable, ()>, ConnectionError> {
        match self {
            Self::Disconnected(_) => unreachable!("can't read() in Disconnected state"),
            Self::Stopped => unreachable!("can't read() in Stopped state"),

            Self::Dns(state) => {
                let Done(next) = state.on_readable(now, config, quic_config, event_loop)? else {
                    return Ok(Pending(()));
                };
                Ok(Done(Readable::NextState(next)))
            }

            Self::Http3(state) => {
                let readable = state.on_readable(now, config)?;
                Ok(readable)
            }
        }
    }

    pub(crate) fn on_writable(&mut self, now: Now) -> Result<(), ConnectionError> {
        match self {
            Self::Disconnected(_) => unreachable!("can't write() in Disconnected state"),
            Self::Stopped => unreachable!("can't write() in Stopped state"),
            Self::Http3(_) => unreachable!("QUIC never waits for writability"),
            Self::Dns(state) => state.on_writable(now),
        }
    }

    pub(crate) fn process_expired_timers(&mut self) -> Result<(), ConnectionError> {
        if let Self::Http3(state) = self {
            state.process_expired_timers()?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> Result<(), ConnectionError> {
        if let Self::Http3(state) = self {
            state.flush()?;
        }
        Ok(())
    }

    pub(crate) fn next_timer(&mut self) -> Option<Instant> {
        match self {
            Self::Http3(state) => state.next_timer(),
            Self::Disconnected(_) | Self::Dns(_) | Self::Stopped => None,
        }
    }

    pub(crate) fn push(&mut self, message: &Message, now: Now) -> bool {
        match self {
            Self::Http3(state) => state.push(message, now),
            Self::Disconnected(_) | Self::Dns(_) | Self::Stopped => false,
        }
    }
}

pub(crate) enum Readable {
    NextState(State),
    Message(Message),
}
