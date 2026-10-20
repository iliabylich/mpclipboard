use crate::{
    Connectivity,
    config::Config,
    connection::{
        error::ConnectionError,
        quic_config::QuicConfig,
        state::{DnsState, FREEZE_TIME_IN_SECS, Http3, State},
    },
};
use core::net::{Ipv4Addr, SocketAddr};
use mpclipboard_shared::{Now, Past};

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisconnectedState(Option<Past>);

impl DisconnectedState {
    pub(crate) const NAME: &str = "Disconnected";
    pub(crate) const CONNECTIVITY: Connectivity = Connectivity::Disconnected;

    pub(crate) const fn new(disconnected_at: Option<Past>) -> Self {
        Self(disconnected_at)
    }

    pub(crate) fn tick(
        &self,
        now: Now,
        config: &Config,
        quic_config: &QuicConfig,
    ) -> Result<Option<State>, ConnectionError> {
        if let Some(disconnected_at) = self.0
            && now.seconds_since(disconnected_at) <= FREEZE_TIME_IN_SECS
        {
            return Ok(None);
        }

        let state = Self::reconnect(now, config, quic_config)?;
        Ok(Some(state))
    }

    fn reconnect(
        now: Now,
        config: &Config,
        quic_config: &QuicConfig,
    ) -> Result<State, ConnectionError> {
        if let Ok(ip) = config.url.host().parse::<Ipv4Addr>() {
            let addr = SocketAddr::from((ip, config.url.port()));
            let state = Http3::connect(now, addr, config, quic_config)?;
            Ok(State::Http3(state))
        } else {
            let state = DnsState::start(now, config)?;
            Ok(State::Dns(state))
        }
    }
}
