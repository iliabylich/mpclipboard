use crate::{
    HostPort, MAX_HOST_LENGTH, MAX_HOST_PORT_LENGTH, NonEmptyInlineString,
    array_writer::ArrayWriter,
};
use anyhow::{Context, Result, bail};
use core::{
    fmt::Write,
    net::{SocketAddr, SocketAddrV4},
};
use std::net::ToSocketAddrs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url {
    tls: bool,
    host: NonEmptyInlineString<MAX_HOST_LENGTH>,
    port: u16,
    header: HostPort,
}

impl Url {
    pub fn parse(url: &str) -> Result<Self> {
        let (scheme, url) = url
            .split_once("://")
            .context("no :// separator in the URL")?;
        let (host, port) = url
            .rsplit_once(':')
            .context("no : separator between host and port")?;

        let tls = match scheme {
            "http" => false,
            "https" => true,
            _ => bail!("unknown URL scheme"),
        };
        let host = NonEmptyInlineString::<MAX_HOST_LENGTH>::new(host).context("invalid host")?;
        let port = port.parse::<u16>().context("invalid port")?;

        let mut buf = [0; MAX_HOST_PORT_LENGTH];
        let mut writer = ArrayWriter::new(&mut buf);
        write!(writer, "{}:{port}", host.as_str()).unwrap_or_else(|_| unreachable!());
        let header = core::str::from_utf8(writer.as_bytes()).unwrap_or_else(|_| {
            unreachable!("concatenation of valid utf8 strings must be a valid utf8 string")
        });
        let header = NonEmptyInlineString::new(header).unwrap_or_else(|_| unreachable!());

        Ok(Self {
            tls,
            host,
            port,
            header,
        })
    }

    pub fn resolve(&self) -> Result<SocketAddrV4> {
        let mut addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .context("failed to resolve URL")?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .context("can't resolve URL to IPv4 address")
    }

    #[must_use]
    pub const fn is_tls(&self) -> bool {
        self.tls
    }

    #[must_use]
    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    pub const fn header(&self) -> HostPort {
        self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let url = Url::parse("http://localhost:3000").unwrap();
        assert!(!url.tls);
        assert_eq!(url.host.as_str(), "localhost");
        assert_eq!(url.port, 3000);
        assert_eq!(url.header.as_str(), "localhost:3000");

        let url = Url::parse("https://google.com:443").unwrap();
        assert!(url.tls);
        assert_eq!(url.host.as_str(), "google.com");
        assert_eq!(url.port, 443);
        assert_eq!(url.header.as_str(), "google.com:443");
    }
}
