use crate::{
    Connectivity,
    config::Config,
    connection::{
        error::ConnectionError,
        quic_config::QuicConfig,
        state::{FREEZE_TIME_IN_SECS, Http3, State},
    },
    event_loop::EventLoop,
};
use core::net::{Ipv4Addr, SocketAddrV4};
use mini_sansio_dns::{Dns as DnsConn, DnsRecordType, DnsWants, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::{Now, Past, RegisteredFd, Wants, prelude::*};
use rustix::{
    io::{Errno, FdFlags},
    net::{AddressFamily, RecvFlags, SendFlags, SocketType},
};

#[derive(Debug)]
pub(crate) struct Dns {
    fd: RegisteredFd,
    conn: DnsConn,
    buf: [u8; MAX_DNS_PACKET_LEN],
    last_activity_at: Past,
}

const DNS_SERVER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53);

impl Dns {
    pub(crate) const NAME: &str = "Dns";
    pub(crate) const CONNECTIVITY: Connectivity = Connectivity::Connecting;

    pub(crate) fn start(
        now: Now,
        config: &Config,
        event_loop: &EventLoop,
    ) -> Result<Self, ConnectionError> {
        let mut buf = [0; MAX_DNS_PACKET_LEN];

        let mut dns = DnsConn::new(config.url.host(), DnsRecordType::A, &mut buf)?;

        let fd = rustix::net::socket(AddressFamily::INET, SocketType::DGRAM, None)
            .map_err(ConnectionError::OpenSocket)?;
        rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectionError::SetCloexec)?;
        rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectionError::SwitchToNonBlocking)?;
        rustix::net::connect(&fd, &DNS_SERVER).map_err(ConnectionError::ConnectToDns)?;
        let fd = event_loop.register(fd, wants(&mut dns, &mut buf))?;

        Ok(Self {
            fd,
            conn: dns,
            buf,
            last_activity_at: now.as_past(),
        })
    }

    pub(crate) fn is_stuck(&self, now: Now) -> bool {
        now.seconds_since(self.last_activity_at) > FREEZE_TIME_IN_SECS
    }

    pub(crate) fn on_readable(
        &mut self,
        now: Now,
        config: &Config,
        quic_config: &QuicConfig,
        event_loop: &EventLoop,
    ) -> Result<Completion<State, ()>, ConnectionError> {
        let Some(DnsWants::Read { buf: response, .. }) = self.conn.wants(&mut self.buf)? else {
            unreachable!("Dns::on_readable() is called only when Dns wants read()")
        };

        match rustix::net::recv(&self.fd, response, RecvFlags::empty()) {
            Ok((len, _)) => {
                let (mut addr, _seq) = self.conn.satisfy_read(len, &self.buf)?;

                addr.set_port(config.url.port());
                let next = Http3::connect(now, addr, config, quic_config, event_loop)?;
                Ok(Done(State::Http3(next)))
            }
            Err(Errno::AGAIN) => {
                self.last_activity_at = now.as_past();
                Ok(Pending(()))
            }
            Err(errno) => Err(ConnectionError::ReadDnsResponse(errno)),
        }
    }

    pub(crate) fn on_writable(&mut self, now: Now) -> Result<(), ConnectionError> {
        let Some(DnsWants::Write { buf: request, .. }) = self.conn.wants(&mut self.buf)? else {
            unreachable!("Dns::on_writable() is called only when Dns wants write()")
        };

        match rustix::net::send(&self.fd, request, SendFlags::empty()) {
            Ok(len) => {
                self.conn.satisfy_write(len, &mut self.buf)?;
                self.fd.modify(wants(&mut self.conn, &mut self.buf))?;
            }
            Err(Errno::AGAIN) => {}
            Err(errno) => return Err(ConnectionError::WriteDnsRequest(errno)),
        }

        self.last_activity_at = now.as_past();

        Ok(())
    }
}

fn wants(conn: &mut DnsConn, buf: &mut [u8; MAX_DNS_PACKET_LEN]) -> Wants {
    match conn.wants(buf) {
        Ok(Some(DnsWants::Write { .. })) => Wants::Write,
        Ok(Some(DnsWants::Read { .. })) => Wants::Read,
        Ok(None) => unreachable!("DnsState never holds a finished Dns"),
        Err(err) => unreachable!("Dns::wants() failed: {err}"),
    }
}
