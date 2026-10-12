use crate::connection::error::ConnectionError;
use core::net::SocketAddr;
use mini_sansio_dns::{Dns, DnsWants, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::prelude::*;
use rustix::{fd::AsFd, io::Errno, net::RecvFlags};

pub fn read_dns_response(
    fd: &impl AsFd,
    dns: &mut Dns,
    buf: &mut [u8; MAX_DNS_PACKET_LEN],
) -> Result<Completion<SocketAddr, ()>, ConnectionError> {
    let Some(DnsWants::Read { buf: response, .. }) = dns.wants(buf)? else {
        unreachable!("read_dns_response() is called only when Dns wants read()")
    };

    let len = match rustix::net::recv(fd, response, RecvFlags::empty()) {
        Ok((len, _)) => len,
        Err(Errno::AGAIN) => return Ok(Pending(())),
        Err(errno) => return Err(ConnectionError::FailedToReadDnsResponse(errno)),
    };

    let (addr, _seq) = dns.satisfy_read(len, buf)?;
    Ok(Done(addr))
}
