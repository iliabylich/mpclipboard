use crate::connection::error::ConnectionError;
use core::net::{Ipv4Addr, SocketAddrV4};
use mini_sansio_dns::{Dns, DnsRecordType, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::Url;
use rustix::{
    fd::OwnedFd,
    io::FdFlags,
    net::{AddressFamily, SocketType},
};

const DNS_SERVER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53);

pub fn start_resolving(
    url: &Url,
    buf: &mut [u8; MAX_DNS_PACKET_LEN],
) -> Result<(OwnedFd, Dns), ConnectionError> {
    let dns = Dns::new(url.host(), DnsRecordType::A, buf)?;

    let fd = rustix::net::socket(AddressFamily::INET, SocketType::DGRAM, None)
        .map_err(ConnectionError::FailedToSocket)?;

    rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectionError::FailedToSetCloexec)?;

    rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectionError::FailedToSwitchToNonBlocking)?;

    rustix::net::connect(&fd, &DNS_SERVER).map_err(ConnectionError::FailedToConnect)?;

    Ok((fd, dns))
}
