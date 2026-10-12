use core::net::{Ipv4Addr, SocketAddrV4};
use mini_sansio_dns::{Dns, DnsError, DnsRecordType, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::Url;
use rustix::{
    fd::OwnedFd,
    io::{Errno, FdFlags},
    net::{AddressFamily, SocketType},
};

const DNS_SERVER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53);

pub fn start_resolving(
    url: &Url,
    buf: &mut [u8; MAX_DNS_PACKET_LEN],
) -> Result<(OwnedFd, Dns), StartResolvingError> {
    let dns = Dns::new(url.host(), DnsRecordType::A, buf)?;

    let fd = rustix::net::socket(AddressFamily::INET, SocketType::DGRAM, None)
        .map_err(StartResolvingError::Socket)?;

    rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(StartResolvingError::SetCloexec)?;

    rustix::io::ioctl_fionbio(&fd, true).map_err(StartResolvingError::SwitchToNonBlocking)?;

    rustix::net::connect(&fd, &DNS_SERVER).map_err(StartResolvingError::Connect)?;

    Ok((fd, dns))
}

#[derive(Debug, thiserror::Error)]
pub enum StartResolvingError {
    #[error("DNS error: {0}")]
    Dns(#[from] DnsError),
    #[error("failed to socket(): {0:?}")]
    Socket(Errno),
    #[error("failed to fcntl(FD_CLOEXEC): {0:?}")]
    SetCloexec(Errno),
    #[error("failed to ioctl(FIONBIO): {0:?}")]
    SwitchToNonBlocking(Errno),
    #[error("failed to connect(): {0:?}")]
    Connect(Errno),
}
