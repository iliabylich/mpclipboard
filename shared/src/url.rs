use crate::{
    HostPort, MAX_HOST_LENGTH, MAX_HOST_PORT_LENGTH, NonEmptyInlineString,
    NonEmptyInlineStringError, array_writer::ArrayWriter,
};
use core::{
    fmt::Write,
    net::{SocketAddr, SocketAddrV4},
    num::ParseIntError,
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
    pub fn parse(url: &str) -> Result<Self, UrlParseError> {
        let (scheme, url) = url
            .split_once("://")
            .ok_or(UrlParseError::NoSchemeSeparator)?;
        let (host, port) = url.rsplit_once(':').ok_or(UrlParseError::NoPortSeparator)?;

        let tls = match scheme {
            "http" => false,
            "https" => true,
            _ => return Err(UrlParseError::UnknownScheme),
        };
        let host = NonEmptyInlineString::new(host).map_err(UrlParseError::InvalidHost)?;
        let port = port.parse::<u16>().map_err(UrlParseError::InvalidPort)?;

        let mut buf = [0; MAX_HOST_PORT_LENGTH];
        let mut writer = ArrayWriter::new(&mut buf);
        write!(writer, "{}:{port}", host.as_str()).unwrap_or_else(|_| {
            unreachable!(
                "host (<= MAX_HOST_LENGTH) + ':' + port (<= 5 digits) fits into MAX_HOST_PORT_LENGTH"
            )
        });
        let header = core::str::from_utf8(writer.as_bytes()).unwrap_or_else(|_| {
            unreachable!("concatenation of valid utf8 strings is a valid utf8 string")
        });
        let header = NonEmptyInlineString::new(header).unwrap_or_else(|_| {
            unreachable!("header is non-empty and fits into MAX_HOST_PORT_LENGTH")
        });

        Ok(Self {
            tls,
            host,
            port,
            header,
        })
    }

    pub fn resolve(&self) -> Result<SocketAddrV4, UrlResolveError> {
        let mut addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(UrlResolveError::Lookup)?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .ok_or(UrlResolveError::NoIPv4Address)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlParseError {
    NoSchemeSeparator,
    NoPortSeparator,
    UnknownScheme,
    InvalidHost(NonEmptyInlineStringError),
    InvalidPort(ParseIntError),
}

impl core::fmt::Display for UrlParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSchemeSeparator => write!(f, "no :// separator in the URL"),
            Self::NoPortSeparator => write!(f, "no : separator between host and port"),
            Self::UnknownScheme => write!(f, "unknown URL scheme"),
            Self::InvalidHost(err) => write!(f, "invalid host: {err}"),
            Self::InvalidPort(err) => write!(f, "invalid port: {err}"),
        }
    }
}

impl core::error::Error for UrlParseError {}

#[derive(Debug)]
pub enum UrlResolveError {
    Lookup(std::io::Error),
    NoIPv4Address,
}

impl core::fmt::Display for UrlResolveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Lookup(err) => write!(f, "failed to resolve URL: {err}"),
            Self::NoIPv4Address => write!(f, "can't resolve URL to IPv4 address"),
        }
    }
}

impl core::error::Error for UrlResolveError {}

#[cfg(test)]
mod tests {
    use super::{Url, UrlParseError};
    use crate::{HostPort, NonEmptyInlineString, NonEmptyInlineStringError};

    #[test]
    fn test_parse() {
        assert_eq!(
            Url::parse("http://localhost:3000"),
            Ok(Url {
                tls: false,
                host: NonEmptyInlineString::const_new("localhost"),
                port: 3000,
                header: HostPort::const_new("localhost:3000"),
            })
        );

        assert_eq!(
            Url::parse("https://google.com:443"),
            Ok(Url {
                tls: true,
                host: NonEmptyInlineString::const_new("google.com"),
                port: 443,
                header: HostPort::const_new("google.com:443"),
            })
        );
    }

    #[test]
    fn test_parse_err() {
        assert_eq!(
            Url::parse("localhost:3000"),
            Err(UrlParseError::NoSchemeSeparator)
        );
        assert_eq!(
            Url::parse("http://localhost"),
            Err(UrlParseError::NoPortSeparator)
        );
        assert_eq!(
            Url::parse("ftp://localhost:3000"),
            Err(UrlParseError::UnknownScheme)
        );
        assert_eq!(
            Url::parse("http://:3000"),
            Err(UrlParseError::InvalidHost(NonEmptyInlineStringError::Empty))
        );
        assert!(matches!(
            Url::parse("http://localhost:99999"),
            Err(UrlParseError::InvalidPort(_))
        ));
    }
}
