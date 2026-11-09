use crate::fixed_str::{FixedStr, FixedStrError};
use core::str::Utf8Error;

pub(crate) type Token = FixedStr<20>;
pub(crate) type ID = FixedStr<20>;

const START_LINE: &str = "GET / HTTP/1.1";
const PROTOCOL_VERSION: &str = "6";
const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";
const USED_HEADERS: [&str; 5] = ["Token", "ID", "Version", "Connection", "Upgrade"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) token: Token,
    pub(crate) id: ID,
}

pub(crate) struct RequestReader {
    line: [u8; Self::LINE_CAPACITY],
    len: usize,
    state: State,
    headers: usize,
    token: Option<Token>,
    id: Option<ID>,
    version: Version,
    connection_upgrade: bool,
    upgrade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    StartLine,
    Header,
    SkippingHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Version {
    Missing,
    Supported,
    Unsupported,
}

impl RequestReader {
    const MAX_LINE_LENGTH: usize = 30;
    const LINE_CAPACITY: usize = Self::MAX_LINE_LENGTH + 2;
    const MAX_HEADERS: usize = 16;

    pub(crate) fn received(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<(Request, usize)>, RequestReaderError> {
        for (index, byte) in bytes.iter().copied().enumerate() {
            if self.state == State::SkippingHeader {
                self.skip(byte)?;
                continue;
            }

            if self.len == Self::LINE_CAPACITY {
                self.overflow()?;
                self.skip(byte)?;
                continue;
            }

            self.line[self.len] = byte;
            self.len += 1;
            if byte != b'\n' {
                continue;
            }

            let len = core::mem::take(&mut self.len);
            if let Some(request) = self.line_received(len)? {
                return Ok(Some((request, index + 1)));
            }
        }

        Ok(None)
    }

    fn skip(&mut self, byte: u8) -> Result<(), RequestReaderError> {
        if byte == b'\n' {
            self.state = State::Header;
            self.count_header()?;
        }
        Ok(())
    }

    fn overflow(&mut self) -> Result<(), RequestReaderError> {
        if self.state == State::StartLine {
            return Err(RequestReaderError::LineTooLong);
        }

        let line = &self.line[..self.len];
        if let Some(colon) = line.iter().position(|byte| *byte == b':') {
            let name = &line[..colon];
            if USED_HEADERS
                .iter()
                .any(|used| name.eq_ignore_ascii_case(used.as_bytes()))
            {
                return Err(RequestReaderError::LineTooLong);
            }
        }

        self.state = State::SkippingHeader;
        self.len = 0;
        Ok(())
    }

    fn count_header(&mut self) -> Result<(), RequestReaderError> {
        self.headers += 1;
        if self.headers > Self::MAX_HEADERS {
            return Err(RequestReaderError::TooManyHeaders);
        }
        Ok(())
    }

    fn line_received(&mut self, len: usize) -> Result<Option<Request>, RequestReaderError> {
        let line = self.line;
        let Some(line) = line[..len].strip_suffix(b"\r\n") else {
            return Err(RequestReaderError::MalformedLine);
        };

        match self.state {
            State::StartLine => {
                if line != START_LINE.as_bytes() {
                    return Err(RequestReaderError::InvalidStartLine);
                }
                self.state = State::Header;
            }
            State::Header if line.is_empty() => return self.finish().map(Some),
            State::Header => {
                self.count_header()?;
                let Some(colon) = line.iter().position(|byte| *byte == b':') else {
                    return Err(RequestReaderError::MalformedHeader);
                };
                let (name, value) = (&line[..colon], line[colon + 1..].trim_ascii());
                if name.is_empty() || name.iter().any(u8::is_ascii_whitespace) {
                    return Err(RequestReaderError::MalformedHeader);
                }
                self.header_received(name, value)?;
            }
            State::SkippingHeader => unreachable!("skipped lines are never buffered"),
        }

        Ok(None)
    }

    fn header_received(&mut self, name: &[u8], value: &[u8]) -> Result<(), RequestReaderError> {
        use RequestReaderError::{InvalidID, InvalidToken};

        let utf8 = || core::str::from_utf8(value);

        if name.eq_ignore_ascii_case(b"Token") {
            self.token = Some(Token::new(utf8()?).map_err(InvalidToken)?);
        } else if name.eq_ignore_ascii_case(b"ID") {
            self.id = Some(ID::new(utf8()?).map_err(InvalidID)?);
        } else if name.eq_ignore_ascii_case(b"Version") {
            self.version = if value == PROTOCOL_VERSION.as_bytes() {
                Version::Supported
            } else {
                Version::Unsupported
            };
        } else if name.eq_ignore_ascii_case(b"Connection") {
            self.connection_upgrade = has_token(value, "upgrade");
        } else if name.eq_ignore_ascii_case(b"Upgrade") {
            self.upgrade = has_token(value, UPGRADE_PROTOCOL);
        }

        Ok(())
    }

    fn finish(&self) -> Result<Request, RequestReaderError> {
        use RequestReaderError::{
            MissingHeader, NotUpgrade, UnsupportedUpgrade, UnsupportedVersion,
        };

        if !self.connection_upgrade {
            return Err(NotUpgrade);
        }
        if !self.upgrade {
            return Err(UnsupportedUpgrade);
        }
        match self.version {
            Version::Missing => return Err(MissingHeader("Version")),
            Version::Unsupported => return Err(UnsupportedVersion),
            Version::Supported => {}
        }

        Ok(Request {
            token: self.token.ok_or(MissingHeader("Token"))?,
            id: self.id.ok_or(MissingHeader("ID"))?,
        })
    }
}

impl Default for RequestReader {
    fn default() -> Self {
        Self {
            line: [0; Self::LINE_CAPACITY],
            len: 0,
            state: State::StartLine,
            headers: 0,
            token: None,
            id: None,
            version: Version::Missing,
            connection_upgrade: false,
            upgrade: false,
        }
    }
}

fn has_token(value: &[u8], token: &str) -> bool {
    value
        .split(|byte| *byte == b',')
        .any(|item| item.trim_ascii().eq_ignore_ascii_case(token.as_bytes()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestReaderError {
    LineTooLong,
    TooManyHeaders,
    MalformedLine,
    MalformedHeader,
    InvalidStartLine,
    NonUtf8(Utf8Error),
    InvalidToken(FixedStrError),
    InvalidID(FixedStrError),
    NotUpgrade,
    UnsupportedUpgrade,
    UnsupportedVersion,
    MissingHeader(&'static str),
}

impl core::fmt::Display for RequestReaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LineTooLong => write!(
                f,
                "request line or used header exceeds {} bytes",
                RequestReader::MAX_LINE_LENGTH
            ),
            Self::TooManyHeaders => write!(
                f,
                "request has more than {} headers",
                RequestReader::MAX_HEADERS
            ),
            Self::MalformedLine => f.write_str("request line doesn't end with CRLF"),
            Self::MalformedHeader => f.write_str("malformed header, expected Name: value"),
            Self::InvalidStartLine => write!(f, "unexpected start line, expected {START_LINE}"),
            Self::NonUtf8(err) => write!(f, "non-utf8 header: {err}"),
            Self::InvalidToken(err) => write!(f, "malformed token: {err}"),
            Self::InvalidID(err) => write!(f, "malformed id: {err}"),
            Self::NotUpgrade => f.write_str("request is not Connection: Upgrade"),
            Self::UnsupportedUpgrade => {
                write!(f, "request is not Upgrade: {UPGRADE_PROTOCOL}")
            }
            Self::UnsupportedVersion => {
                write!(
                    f,
                    "unsupported protocol version, expected={PROTOCOL_VERSION}"
                )
            }
            Self::MissingHeader(name) => write!(f, "missing {name} header"),
        }
    }
}

impl From<Utf8Error> for RequestReaderError {
    fn from(err: Utf8Error) -> Self {
        Self::NonUtf8(err)
    }
}

#[cfg(test)]
mod tests {
    use super::{ID, Request, RequestReader, RequestReaderError, Token};
    use crate::fixed_str::FixedStrError;
    use std::{format, string::String, vec::Vec};

    const REQUEST: &str = "GET / HTTP/1.1\r\n\
Host: localhost:8443\r\n\
User-Agent: mpclipboard\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
Id: test-client\r\n\
Token: sekret\r\n\
Version: 6\r\n\
Via: 1.1 Caddy\r\n\
X-Forwarded-For: 127.0.0.1\r\n\
X-Forwarded-Host: localhost:8443\r\n\
X-Forwarded-Proto: https\r\n\
Accept-Encoding: gzip\r\n\
\r\n";

    type Parsed = Result<Option<(Request, Vec<u8>)>, RequestReaderError>;

    fn req() -> Request {
        Request {
            token: Token::new("sekret").unwrap(),
            id: ID::new("test-client").unwrap(),
        }
    }

    fn read_in_chunks(bytes: &[u8], chunk_size: usize) -> Parsed {
        let mut reader = RequestReader::default();
        let mut offset = 0;
        for chunk in bytes.chunks(chunk_size) {
            if let Some((request, consumed)) = reader.received(chunk)? {
                return Ok(Some((request, bytes[offset + consumed..].to_vec())));
            }
            offset += chunk.len();
        }
        Ok(None)
    }

    fn read_all(bytes: &str) -> Parsed {
        let bytes = bytes.as_bytes();
        let whole = read_in_chunks(bytes, bytes.len());
        for chunk_size in [1, 3, 20] {
            assert_eq!(
                read_in_chunks(bytes, chunk_size),
                whole,
                "chunk size {chunk_size}"
            );
        }
        whole
    }

    fn with_header(header: &str) -> String {
        REQUEST.replacen("\r\n\r\n", &format!("\r\n{header}\r\n\r\n"), 1)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(REQUEST), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{REQUEST}abc");
        assert_eq!(read_all(&bytes), Ok(Some((req(), b"abc".to_vec()))));
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(read_all(&REQUEST[..REQUEST.len() - 2]), Ok(None));
    }

    #[test]
    fn test_connection_list() {
        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: close, upgrade");
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_not_upgrade() {
        let bytes = REQUEST.replace("Connection: Upgrade\r\n", "");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::NotUpgrade));

        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::NotUpgrade));
    }

    #[test]
    fn test_unsupported_upgrade() {
        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw\r\n", "");
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::UnsupportedUpgrade)
        );

        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw", "Upgrade: websocket");
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::UnsupportedUpgrade)
        );
    }

    #[test]
    fn test_missing_header() {
        let bytes = REQUEST.replace("Id: test-client\r\n", "");
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::MissingHeader("ID"))
        );

        let bytes = REQUEST.replace("Version: 6\r\n", "");
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::MissingHeader("Version"))
        );
    }

    #[test]
    fn test_unsupported_version() {
        let bytes = REQUEST.replace("Version: 6", "Version: 5");
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::UnsupportedVersion)
        );
    }

    #[test]
    fn test_invalid_start_line() {
        let bytes = REQUEST.replace("GET / HTTP/1.1", "POST / HTTP/1.1");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::InvalidStartLine));
    }

    #[test]
    fn test_start_line_too_long() {
        let bytes = REQUEST.replace("GET / HTTP/1.1", "GET /aaaaaaaaaaaaaaaaaaaa HTTP/1.1");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::LineTooLong));
    }

    #[test]
    fn test_token_max_len() {
        let token = "a".repeat(20);
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {token}"));
        let parsed = read_all(&bytes).unwrap().unwrap();
        assert_eq!(parsed.0.token, Token::new(&token).unwrap());
    }

    #[test]
    fn test_token_too_long_but_line_fits() {
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {}", "a".repeat(23)));
        assert_eq!(
            read_all(&bytes),
            Err(RequestReaderError::InvalidToken(FixedStrError::TooLong {
                len: 23
            }))
        );
    }

    #[test]
    fn test_used_header_line_too_long() {
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {}", "a".repeat(24)));
        assert_eq!(read_all(&bytes), Err(RequestReaderError::LineTooLong));

        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive, Upgrade");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::LineTooLong));
    }

    #[test]
    fn test_long_unused_header_is_skipped() {
        let bytes = with_header(&format!("X-Long: {}", "a".repeat(300)));
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_long_unused_header_name_is_skipped() {
        let bytes = with_header(&format!("X-{}: 1", "a".repeat(100)));
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_used_header_after_skipped_one() {
        let bytes = REQUEST.replace("Token: sekret\r\n", "").replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\nToken: sekret\r\n\r\n", "a".repeat(100)),
            1,
        );
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_garbage_start_line() {
        assert_eq!(
            read_all("boo\r\n\r\n"),
            Err(RequestReaderError::InvalidStartLine)
        );
    }

    #[test]
    fn test_lf_without_cr() {
        let bytes = REQUEST.replacen("\r\n", "\n", 1);
        assert_eq!(read_all(&bytes), Err(RequestReaderError::MalformedLine));

        let bytes = REQUEST.replace("Version: 6\r\n", "Version: 6\n");
        assert_eq!(read_all(&bytes), Err(RequestReaderError::MalformedLine));
    }

    #[test]
    fn test_malformed_header() {
        assert_eq!(
            read_all(&with_header("no colon")),
            Err(RequestReaderError::MalformedHeader)
        );
        assert_eq!(
            read_all(&with_header(": empty name")),
            Err(RequestReaderError::MalformedHeader)
        );
        assert_eq!(
            read_all(&with_header("Token : sekret")),
            Err(RequestReaderError::MalformedHeader)
        );
    }

    #[test]
    fn test_header_value_whitespace() {
        let bytes = REQUEST.replace("Token: sekret", "Token:\t sekret  ");
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));

        let bytes = REQUEST.replace("Token: sekret", "Token:sekret");
        assert_eq!(read_all(&bytes), Ok(Some((req(), Vec::new()))));
    }

    fn with_extra_headers(count: usize) -> String {
        let extra: String = (0..count).map(|i| format!("X-Extra-{i}: 1\r\n")).collect();
        REQUEST.replacen("\r\n\r\n", &format!("\r\n{extra}\r\n"), 1)
    }

    #[test]
    fn test_max_headers() {
        assert_eq!(
            read_all(&with_extra_headers(4)),
            Ok(Some((req(), Vec::new())))
        );
    }

    #[test]
    fn test_too_many_headers() {
        assert_eq!(
            read_all(&with_extra_headers(5)),
            Err(RequestReaderError::TooManyHeaders)
        );
    }

    #[test]
    fn test_skipped_headers_count() {
        let long = format!("X-Long: {}\r\n", "a".repeat(100));
        let extra = long.repeat(5);
        let bytes = REQUEST.replacen("\r\n\r\n", &format!("\r\n{extra}\r\n"), 1);
        assert_eq!(read_all(&bytes), Err(RequestReaderError::TooManyHeaders));
    }
}
