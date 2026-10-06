use crate::{
    HostPort, MaxHostLength, MaxHostPortLength, NonEmptyInlineString, NonEmptyInlineStringError,
    array_writer::ArrayWriter,
};
use core::{fmt::Write, num::ParseIntError};
use typenum::Unsigned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    tls: bool,
    host: NonEmptyInlineString<MaxHostLength>,
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
        let host = NonEmptyInlineString::new(host)?;
        let port = port.parse::<u16>()?;

        let mut buf = [0; MaxHostPortLength::USIZE];
        let mut writer = ArrayWriter::new(&mut buf);
        write!(writer, "{}:{port}", host.as_str()).unwrap_or_else(|_| {
            unreachable!(
                "host (<= MaxHostLength) + ':' + port (<= 5 digits) fits into MaxHostPortLength"
            )
        });
        let header = core::str::from_utf8(writer.as_bytes()).unwrap_or_else(|_| {
            unreachable!("concatenation of valid utf8 strings is a valid utf8 string")
        });
        let header = NonEmptyInlineString::new(header).unwrap_or_else(|_| {
            unreachable!("header is non-empty and fits into MaxHostPortLength")
        });

        Ok(Self {
            tls,
            host,
            port,
            header,
        })
    }

    #[must_use]
    pub const fn is_tls(&self) -> bool {
        self.tls
    }

    #[must_use]
    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    pub const fn header(&self) -> &HostPort {
        &self.header
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlParseError {
    #[error("no :// separator in the URL")]
    NoSchemeSeparator,
    #[error("no : separator between host and port")]
    NoPortSeparator,
    #[error("unknown URL scheme")]
    UnknownScheme,
    #[error("invalid host: {0}")]
    InvalidHost(#[from] NonEmptyInlineStringError),
    #[error("invalid port: {0}")]
    InvalidPort(#[from] ParseIntError),
}

#[cfg(test)]
mod tests {
    use super::{Url, UrlParseError};
    use crate::{HostPort, NonEmptyInlineString, NonEmptyInlineStringError};
    use core::assert_matches;

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
        assert_matches!(
            Url::parse("http://localhost:99999"),
            Err(UrlParseError::InvalidPort(_))
        );
    }
}
