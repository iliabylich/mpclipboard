use crate::{NonEmptyString, NonEmptyStringError};
use alloc::format;
use core::num::{IntErrorKind, ParseIntError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    tls: bool,
    host: NonEmptyString,
    port: u16,
    header: NonEmptyString,
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
        let host = NonEmptyString::new(host)?;
        let port = port.parse::<u16>()?;

        let header = NonEmptyString::new(format!("{host}:{port}"))
            .unwrap_or_else(|_| unreachable!("header always contains ':'"));

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
    pub const fn host(&self) -> &str {
        self.host.as_str()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub const fn header(&self) -> &NonEmptyString {
        &self.header
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UrlParseError {
    #[error("no :// separator in the URL")]
    NoSchemeSeparator,
    #[error("no : separator between host and port")]
    NoPortSeparator,
    #[error("unknown URL scheme")]
    UnknownScheme,
    #[error("invalid host: {0}")]
    InvalidHost(#[from] NonEmptyStringError),
    #[error("invalid port: {0:?}")]
    InvalidPort(IntErrorKind),
}

impl From<ParseIntError> for UrlParseError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidPort(*err.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::{Url, UrlParseError};
    use crate::{NonEmptyString, NonEmptyStringError};
    use core::num::IntErrorKind;

    #[test]
    fn test_parse() {
        assert_eq!(
            Url::parse("http://localhost:3000"),
            Ok(Url {
                tls: false,
                host: NonEmptyString::new("localhost").unwrap(),
                port: 3000,
                header: NonEmptyString::new("localhost:3000").unwrap(),
            })
        );

        assert_eq!(
            Url::parse("https://google.com:443"),
            Ok(Url {
                tls: true,
                host: NonEmptyString::new("google.com").unwrap(),
                port: 443,
                header: NonEmptyString::new("google.com:443").unwrap(),
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
            Err(UrlParseError::InvalidHost(NonEmptyStringError::Empty))
        );
        assert_eq!(
            Url::parse("http://localhost:99999"),
            Err(UrlParseError::InvalidPort(IntErrorKind::PosOverflow))
        );
    }
}
