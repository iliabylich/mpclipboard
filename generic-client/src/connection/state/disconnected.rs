use crate::{
    Connectivity,
    config::Config,
    connection::{
        error::ConnectionError,
        state::{DnsState, FREEZE_TIME_IN_SECS, State, TcpState},
    },
};
use core::net::{Ipv4Addr, SocketAddr};
use mpclipboard_shared::Wants;
use rustix::fd::BorrowedFd;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisconnectedState(Option<u64>);

impl DisconnectedState {
    pub(crate) const fn new(disconnected_at: Option<u64>) -> Self {
        Self(disconnected_at)
    }

    pub(crate) const fn name() -> &'static str {
        "Disconnected"
    }

    pub(crate) const fn connectivity() -> Connectivity {
        Connectivity::Disconnected
    }

    pub(crate) const fn wants() -> Option<(BorrowedFd<'static>, Wants)> {
        None
    }

    pub(crate) fn tick(&self, now: u64, config: &Config) -> Result<Option<State>, ConnectionError> {
        if let Some(disconnected_at) = self.0 {
            let seconds_passed = now
                .checked_sub(disconnected_at)
                .unwrap_or_else(|| unreachable!("time goes backwards"));

            if seconds_passed <= FREEZE_TIME_IN_SECS {
                return Ok(None);
            }
        }

        let state = Self::reconnect(now, config)?;
        Ok(Some(state))
    }

    fn reconnect(now: u64, config: &Config) -> Result<State, ConnectionError> {
        if let Ok(ip) = config.url.host().parse::<Ipv4Addr>() {
            let addr = SocketAddr::from((ip, config.url.port()));
            TcpState::connect(now, addr, config)
        } else {
            let state = DnsState::start(now, config)?;
            Ok(State::Dns(state))
        }
    }
}
