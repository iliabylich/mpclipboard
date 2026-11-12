use crate::{constants::DNS_SERVER, runtime::LoopContext};
use core::net::{Ipv4Addr, SocketAddr};
use externally_driven_async_rt::net::UdpSocket;
use mini_sansio_dns::{Dns, DnsError, DnsRecordType, DnsWants, MAX_DNS_PACKET_LEN};
use rustix::io::Errno;

pub(crate) async fn resolve(ctx: &LoopContext<'_>, host: &str) -> Result<Ipv4Addr, ResolveError> {
    let mut buf = [0; MAX_DNS_PACKET_LEN];
    let mut dns = Dns::new(host, DnsRecordType::A, &mut buf)?;

    let mut socket = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .map_err(ResolveError::Bind)?;
    socket
        .connect(SocketAddr::V4(DNS_SERVER))
        .map_err(ResolveError::Connect)?;

    loop {
        match dns.wants(&mut buf)? {
            Some(DnsWants::Write { buf: request, .. }) => {
                let len = socket
                    .send(ctx, request)
                    .await
                    .map_err(ResolveError::Send)?;
                dns.satisfy_write(len, &mut buf)?;
            }
            Some(DnsWants::Read { buf: response, .. }) => {
                let len = socket
                    .recv(ctx, response)
                    .await
                    .map_err(ResolveError::Recv)?;
                let (addr, _seq) = dns.satisfy_read(len, &buf)?;
                return match addr {
                    SocketAddr::V4(addr) => Ok(*addr.ip()),
                    SocketAddr::V6(_) => Err(ResolveError::NotIpv4),
                };
            }
            None => unreachable!("Dns finishes only in satisfy_read(), which returns"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResolveError {
    #[error("DNS error: {0}")]
    Dns(#[from] DnsError),
    #[error("failed to bind DNS socket: {0}")]
    Bind(Errno),
    #[error("failed to connect to DNS server: {0}")]
    Connect(Errno),
    #[error("failed to send DNS request: {0}")]
    Send(Errno),
    #[error("failed to receive DNS response: {0}")]
    Recv(Errno),
    #[error("DNS server returned a non-IPv4 address")]
    NotIpv4,
}
