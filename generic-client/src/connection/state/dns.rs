use crate::{
    Connectivity,
    config::Config,
    connection::{
        actions::{read_dns_response, start_resolving, write_dns_request},
        error::ConnectionError,
        quic_config::QuicConfig,
        state::{FREEZE_TIME_IN_SECS, Http3, State},
    },
};
use mini_sansio_dns::{Dns, DnsWants, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::{Wants, prelude::*};
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};

#[derive(Debug)]
pub(crate) struct DnsState {
    fd: OwnedFd,
    dns: Dns,
    buf: [u8; MAX_DNS_PACKET_LEN],
    last_activity_at: u64,
}

impl DnsState {
    pub(crate) const NAME: &str = "Dns";
    pub(crate) const CONNECTIVITY: Connectivity = Connectivity::Connecting;

    pub(crate) fn start(now: u64, config: &Config) -> Result<Self, ConnectionError> {
        let mut buf = [0; MAX_DNS_PACKET_LEN];
        let (fd, dns) = start_resolving(&config.url, &mut buf)?;

        Ok(Self {
            fd,
            dns,
            buf,
            last_activity_at: now,
        })
    }

    pub(crate) fn is_stuck(&self, now: u64) -> bool {
        let seconds_passed = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));

        seconds_passed > FREEZE_TIME_IN_SECS
    }

    pub(crate) fn on_readable(
        &mut self,
        now: u64,
        config: &Config,
        quic_config: &QuicConfig,
    ) -> Result<Completion<State, ()>, ConnectionError> {
        let Done(mut addr) = read_dns_response(&self.fd, &mut self.dns, &mut self.buf)? else {
            self.last_activity_at = now;
            return Ok(Pending(()));
        };

        addr.set_port(config.url.port());
        let next = Http3::connect(now, addr, config, quic_config)?;
        Ok(Done(State::Http3(next)))
    }

    pub(crate) fn on_writable(&mut self, now: u64) -> Result<(), ConnectionError> {
        write_dns_request(&self.fd, &mut self.dns, &mut self.buf)?;
        self.last_activity_at = now;
        Ok(())
    }

    pub(crate) fn wants(&mut self) -> (BorrowedFd<'_>, Wants) {
        let wants = match self.dns.wants(&mut self.buf) {
            Ok(Some(DnsWants::Write { .. })) => Wants::Write,
            Ok(Some(DnsWants::Read { .. })) => Wants::Read,
            Ok(None) => unreachable!("DnsState never holds a finished Dns"),
            Err(err) => unreachable!("Dns::wants() failed: {err}"),
        };
        (self.fd.as_fd(), wants)
    }
}
