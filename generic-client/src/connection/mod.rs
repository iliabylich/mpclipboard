use crate::event_loop::Epoch;
use crate::{Connectivity, config::Config};
use mpclipboard_shared::{Message, Now, Wants, prelude::*};
use rustix::fd::BorrowedFd;
use std::time::Instant;

mod error;

mod http3_connection;

mod quic_config;
use quic_config::QuicConfig;
pub use quic_config::QuicConfigError;

mod state;
use state::{Disconnected, Readable, State};

pub(crate) struct Connection {
    epoch: Epoch,
    state: State,
    quic_config: QuicConfig,
}

impl Connection {
    pub(crate) fn new() -> Result<Self, QuicConfigError> {
        Ok(Self {
            epoch: Epoch::ZERO,
            state: State::Disconnected(Disconnected::new(None)),
            quic_config: QuicConfig::new()?,
        })
    }

    pub(crate) fn disconnect(&mut self, now: Now) {
        self.state = State::Disconnected(Disconnected::new(Some(now.as_past())));
        self.epoch.bump();
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self.state, State::Disconnected { .. } | State::Stopped)
    }

    pub(crate) fn tick(&mut self, now: Now, config: &Config) {
        if let Some(next) = self.state.tick(now, config, &self.quic_config) {
            self.state = next;
            self.epoch.bump();
        }
    }

    pub(crate) fn push(&mut self, message: &Message, now: Now) -> bool {
        self.state.push(message, now)
    }

    pub(crate) fn on_readable(&mut self, now: Now, config: &Config) -> Option<Message> {
        log::trace!("{self:?} reading...");

        match self.state.on_readable(now, config, &self.quic_config) {
            Ok(Done(Readable::NextState(next))) => {
                self.state = next;
                self.epoch.bump();
                None
            }
            Ok(Done(Readable::Message(message))) => Some(message),
            Ok(Pending(())) => None,
            Err(err) => {
                log::error!("{err}");
                self.disconnect(now);
                None
            }
        }
    }

    pub(crate) fn on_writable(&mut self, now: Now) {
        log::trace!("{self:?} writing...");

        if let Err(err) = self.state.on_writable(now) {
            log::error!("{err}");
            self.disconnect(now);
        }
    }

    pub(crate) fn process_expired_timers(&mut self, now: Now) {
        if let Err(err) = self.state.process_expired_timers() {
            log::error!("{err}");
            self.disconnect(now);
        }
    }

    pub(crate) fn flush(&mut self, now: Now) {
        if let Err(err) = self.state.flush() {
            log::error!("{err}");
            self.disconnect(now);
        }
    }

    pub(crate) fn next_timer(&mut self) -> Option<Instant> {
        self.state.next_timer()
    }

    pub(crate) fn wants(&mut self) -> Option<(BorrowedFd<'_>, Epoch, Wants)> {
        let (fd, wants) = self.state.wants()?;
        Some((fd, self.epoch, wants))
    }
}

impl core::fmt::Debug for Connection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Connection")
            .field("epoch", &self.epoch)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
