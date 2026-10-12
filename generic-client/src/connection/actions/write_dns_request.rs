use crate::connection::error::ConnectionError;
use mini_sansio_dns::{Dns, DnsWants, MAX_DNS_PACKET_LEN};
use rustix::{fd::AsFd, io::Errno, net::SendFlags};

pub fn write_dns_request(
    fd: &impl AsFd,
    dns: &mut Dns,
    buf: &mut [u8; MAX_DNS_PACKET_LEN],
) -> Result<(), ConnectionError> {
    let Some(DnsWants::Write { buf: request, .. }) = dns.wants(buf)? else {
        unreachable!("write_dns_request() is called only when Dns wants write()")
    };

    let len = match rustix::net::send(fd, request, SendFlags::empty()) {
        Ok(len) => len,
        Err(Errno::AGAIN) => return Ok(()),
        Err(errno) => return Err(ConnectionError::FailedToWriteDnsRequest(errno)),
    };

    dns.satisfy_write(len, buf)?;
    Ok(())
}
