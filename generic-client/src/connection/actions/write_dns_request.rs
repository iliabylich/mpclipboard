use mini_sansio_dns::{Dns, DnsError, DnsWants, MAX_DNS_PACKET_LEN};
use rustix::{fd::AsFd, io::Errno, net::SendFlags};

pub(crate) fn write_dns_request(
    fd: &impl AsFd,
    dns: &mut Dns,
    buf: &mut [u8; MAX_DNS_PACKET_LEN],
) -> Result<(), WriteDnsRequestError> {
    let Some(DnsWants::Write { buf: request, .. }) = dns.wants(buf)? else {
        unreachable!("write_dns_request() is called only when Dns wants write()")
    };

    let len = match rustix::net::send(fd, request, SendFlags::empty()) {
        Ok(len) => len,
        Err(Errno::AGAIN) => return Ok(()),
        Err(errno) => return Err(WriteDnsRequestError::Send(errno)),
    };

    dns.satisfy_write(len, buf)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WriteDnsRequestError {
    #[error("DNS error: {0}")]
    Dns(#[from] DnsError),
    #[error("failed to send(): {0:?}")]
    Send(Errno),
}
