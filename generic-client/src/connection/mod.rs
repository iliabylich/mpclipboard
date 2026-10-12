use crate::{Connectivity, config::Config};
use mpclipboard_shared::{Epoch, Message, Wants, prelude::*};
use rustix::fd::BorrowedFd;

mod actions;

mod error;

mod maybe_tls_stream;

mod std_read_write_fd;

mod state;
use state::{DisconnectedState, Readable, State, TcpState};

#[derive(Debug)]
pub struct Connection {
    epoch: Epoch,
    state: State,
}

impl Connection {
    pub(crate) const fn new() -> Self {
        Self {
            epoch: Epoch::new(),
            state: State::Disconnected(DisconnectedState::new(None)),
        }
    }

    pub(crate) fn disconnect(&mut self, now: u64) {
        self.state = State::Disconnected(DisconnectedState::new(Some(now)));
        self.epoch.bump();
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self.state, State::Disconnected { .. })
    }

    pub(crate) fn tick(&mut self, now: u64, config: &Config) {
        if let Some(next) = self.state.tick(now, config) {
            self.state = next;
            self.epoch.bump();
        }
    }

    pub(crate) fn push(&mut self, message: &Message) -> bool {
        let State::Tcp { state: active, .. } = &mut self.state else {
            return false;
        };
        let TcpState::Connected(_reader, writer) = active else {
            return false;
        };
        writer.push(message);
        true
    }

    pub(crate) fn on_readable(&mut self, now: u64, config: &Config) -> Option<Message> {
        log::trace!("{self:?} reading...");

        match self.state.on_readable(now, config) {
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

    pub(crate) fn on_writable(&mut self, now: u64, config: &Config) {
        log::trace!("{self:?} writing...");

        if let Err(err) = self.state.on_writable(now, config) {
            log::error!("{err}");
            self.disconnect(now);
        }
    }

    pub(crate) fn wants(&mut self) -> Option<(BorrowedFd<'_>, Epoch, Wants)> {
        let (fd, wants) = self.state.wants()?;
        Some((fd, self.epoch, wants))
    }
}
