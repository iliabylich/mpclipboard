use crate::{
    HostPort, MAX_HOST_LENGTH, MAX_HOST_PORT_LENGTH, NonEmptyInlineString,
    array_writer::ArrayWriter,
};
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
    pub fn parse(url: &str) -> Result<Self, UrlError> {
        let (scheme, url) = url
            .split_once("://")
            .ok_or(UrlError::MissingSchemeSeparator)?;
        let (host, port) = url.rsplit_once(':').ok_or(UrlError::MissingPortSeparator)?;

        let tls = match scheme {
            "http" => false,
            "https" => true,
            _ => return Err(UrlError::UnknownScheme),
        };
        let host = NonEmptyInlineString::<MAX_HOST_LENGTH>::new(host)
            .map_err(|_| UrlError::InvalidHost)?;
        let port = port.parse::<u16>().map_err(|_| UrlError::InvalidPort)?;

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

    pub fn resolve(&self) -> Result<SocketAddrV4, UrlError> {
        let mut addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|_| UrlError::ResolveFailed)?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .ok_or(UrlError::NoIpv4Address)
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

#[derive(Debug)]
pub enum UrlError {
    MissingSchemeSeparator,
    MissingPortSeparator,
    UnknownScheme,
    InvalidHost,
    InvalidPort,
    ResolveFailed,
    NoIpv4Address,
}

impl core::fmt::Display for UrlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingSchemeSeparator => f.write_str("no :// separator in the URL"),
            Self::MissingPortSeparator => f.write_str("no : separator between host and port"),
            Self::UnknownScheme => f.write_str("unknown URL scheme"),
            Self::InvalidHost => f.write_str("host is empty or too long"),
            Self::InvalidPort => f.write_str("invalid port"),
            Self::ResolveFailed => f.write_str("failed to resolve URL"),
            Self::NoIpv4Address => f.write_str("can't resolve URL to IPv4 address"),
        }
    }
}

impl core::error::Error for UrlError {}

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
