use core::num::{IntErrorKind, ParseIntError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Url {
    host: String,
    port: u16,
}

impl Url {
    pub(crate) fn parse(url: &str) -> Result<Self, UrlParseError> {
        let (scheme, url) = url
            .split_once("://")
            .ok_or(UrlParseError::NoSchemeSeparator)?;
        if scheme != "https" {
            return Err(UrlParseError::NotHttps);
        }
        let (host, port) = url.rsplit_once(':').ok_or(UrlParseError::NoPortSeparator)?;
        if host.is_empty() {
            return Err(UrlParseError::EmptyHost);
        }
        let port = port.parse::<u16>()?;

        Ok(Self {
            host: host.to_string(),
            port,
        })
    }

    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) const fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UrlParseError {
    #[error("no :// separator in the URL")]
    NoSchemeSeparator,
    #[error("url must have https scheme")]
    NotHttps,
    #[error("no : separator between host and port")]
    NoPortSeparator,
    #[error("host is empty")]
    EmptyHost,
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
    use core::num::IntErrorKind;

    #[test]
    fn test_parse() {
        assert_eq!(
            Url::parse("https://google.com:443"),
            Ok(Url {
                host: String::from("google.com"),
                port: 443,
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
            Url::parse("http://localhost:3000"),
            Err(UrlParseError::NotHttps)
        );
        assert_eq!(
            Url::parse("https://localhost"),
            Err(UrlParseError::NoPortSeparator)
        );
        assert_eq!(Url::parse("https://:3000"), Err(UrlParseError::EmptyHost));
        assert_eq!(
            Url::parse("https://localhost:99999"),
            Err(UrlParseError::InvalidPort(IntErrorKind::PosOverflow))
        );
    }
}
