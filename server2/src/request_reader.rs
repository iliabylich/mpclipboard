use crate::fixed_str::{FixedStr, FixedStrError};
use core::str::Utf8Error;

pub(crate) type Token = FixedStr<255>;
pub(crate) type ID = FixedStr<255>;

const PROTOCOL_VERSION: &str = "6";
const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) token: Token,
    pub(crate) id: ID,
}

pub(crate) struct RequestReader {
    buf: [u8; Self::MAX_REQUEST_SIZE],
    len: usize,
}

impl RequestReader {
    const MAX_REQUEST_SIZE: usize = 2048;
    const MAX_HEADERS: usize = 16;

    pub(crate) fn unfilled(&mut self) -> &mut [u8] {
        &mut self.buf[self.len..]
    }

    pub(crate) fn filled(
        &mut self,
        n: usize,
    ) -> Result<Option<(Request, &[u8])>, RequestReaderError> {
        let remaining = Self::MAX_REQUEST_SIZE - self.len;
        if n > remaining {
            return Err(RequestReaderError::FilledTooMuch {
                filled: n,
                remaining,
            });
        }
        self.len += n;

        let mut headers = [httparse::EMPTY_HEADER; Self::MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let httparse::Status::Complete(parsed) = req.parse(&self.buf[..self.len])? else {
            if self.len == Self::MAX_REQUEST_SIZE {
                return Err(RequestReaderError::TooLarge);
            }
            return Ok(None);
        };

        let request = parse_request(&req)?;
        Ok(Some((request, &self.buf[parsed..self.len])))
    }
}

impl Default for RequestReader {
    fn default() -> Self {
        Self {
            buf: [0; Self::MAX_REQUEST_SIZE],
            len: 0,
        }
    }
}

fn parse_request(req: &httparse::Request<'_, '_>) -> Result<Request, RequestReaderError> {
    use RequestReaderError::{
        InvalidID, InvalidStartLine, InvalidToken, MissingHeader, NotUpgrade, UnsupportedUpgrade,
        UnsupportedVersion,
    };

    if req.method != Some("GET") || req.path != Some("/") || req.version != Some(1) {
        return Err(InvalidStartLine);
    }

    let mut token = None;
    let mut id = None;
    let mut version = None;
    let mut connection_upgrade = false;
    let mut upgrade = false;

    for header in req.headers.iter() {
        let name = header.name;
        let value = || core::str::from_utf8(header.value);

        if name.eq_ignore_ascii_case("Token") {
            token = Some(Token::new(value()?).map_err(InvalidToken)?);
        } else if name.eq_ignore_ascii_case("ID") {
            id = Some(ID::new(value()?).map_err(InvalidID)?);
        } else if name.eq_ignore_ascii_case("Version") {
            version = Some(value()?);
        } else if name.eq_ignore_ascii_case("Connection") {
            connection_upgrade = has_token(header.value, "upgrade");
        } else if name.eq_ignore_ascii_case("Upgrade") {
            upgrade = has_token(header.value, UPGRADE_PROTOCOL);
        }
    }

    if !connection_upgrade {
        return Err(NotUpgrade);
    }
    if !upgrade {
        return Err(UnsupportedUpgrade);
    }

    let version = version.ok_or(MissingHeader("Version"))?;
    if version != PROTOCOL_VERSION {
        return Err(UnsupportedVersion);
    }

    Ok(Request {
        token: token.ok_or(MissingHeader("Token"))?,
        id: id.ok_or(MissingHeader("ID"))?,
    })
}

fn has_token(value: &[u8], token: &str) -> bool {
    value
        .split(|byte| *byte == b',')
        .any(|item| item.trim_ascii().eq_ignore_ascii_case(token.as_bytes()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestReaderError {
    Parse(httparse::Error),
    TooLarge,
    FilledTooMuch { filled: usize, remaining: usize },
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
            Self::Parse(err) => write!(f, "malformed HTTP request: {err}"),
            Self::TooLarge => write!(
                f,
                "HTTP request exceeds {} bytes",
                RequestReader::MAX_REQUEST_SIZE
            ),
            Self::FilledTooMuch { filled, remaining } => write!(
                f,
                "filled() reported {filled} bytes, but only {remaining} bytes remained"
            ),
            Self::InvalidStartLine => f.write_str("unexpected start line, expected GET / HTTP/1.1"),
            Self::NonUtf8(err) => write!(f, "non-utf8 header: {err}"),
            Self::InvalidToken(err) => write!(f, "malformed token: {err}"),
            Self::InvalidID(err) => write!(f, "malformed id: {err}"),
            Self::NotUpgrade => f.write_str("request is not Connection: Upgrade"),
            Self::UnsupportedUpgrade => {
                write!(f, "request is not Upgrade: {UPGRADE_PROTOCOL}")
            }
            Self::UnsupportedVersion => {
                write!(f, "unsupported protocol version, expected={PROTOCOL_VERSION}")
            }
            Self::MissingHeader(name) => write!(f, "missing {name} header"),
        }
    }
}

impl From<httparse::Error> for RequestReaderError {
    fn from(err: httparse::Error) -> Self {
        Self::Parse(err)
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
    use std::{format, string::String, vec::Vec};

    const CHUNK_SIZE: usize = 20;

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

    fn req() -> Request {
        Request {
            token: Token::new("sekret").unwrap(),
            id: ID::new("test-client").unwrap(),
        }
    }

    fn feed(
        reader: &mut RequestReader,
        bytes: &[u8],
    ) -> Result<Option<(Request, Vec<u8>)>, RequestReaderError> {
        let unfilled = reader.unfilled();
        let n = bytes.len().min(unfilled.len());
        unfilled[..n].copy_from_slice(&bytes[..n]);
        let parsed = reader.filled(n)?;
        Ok(parsed.map(|(request, leftover)| (request, leftover.to_vec())))
    }

    fn read_all(bytes: &[u8]) -> Result<Option<(Request, Vec<u8>)>, RequestReaderError> {
        let (head, trailer) = bytes.split_at(bytes.len() - CHUNK_SIZE);
        let mut reader = RequestReader::default();
        for chunk in head.chunks(CHUNK_SIZE) {
            assert_eq!(feed(&mut reader, chunk), Ok(None));
        }
        feed(&mut reader, trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(REQUEST.as_bytes()), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{REQUEST}abc");
        assert_eq!(read_all(bytes.as_bytes()), Ok(Some((req(), b"abc".to_vec()))));
    }

    #[test]
    fn test_connection_list() {
        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive, upgrade");
        assert_eq!(read_all(bytes.as_bytes()), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_not_upgrade() {
        let bytes = REQUEST.replace("Connection: Upgrade\r\n", "");
        assert_eq!(read_all(bytes.as_bytes()), Err(RequestReaderError::NotUpgrade));

        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive");
        assert_eq!(read_all(bytes.as_bytes()), Err(RequestReaderError::NotUpgrade));
    }

    #[test]
    fn test_unsupported_upgrade() {
        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw\r\n", "");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::UnsupportedUpgrade)
        );

        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw", "Upgrade: websocket");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::UnsupportedUpgrade)
        );
    }

    #[test]
    fn test_missing_header() {
        let bytes = REQUEST.replace("Id: test-client\r\n", "");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::MissingHeader("ID"))
        );
    }

    #[test]
    fn test_unsupported_version() {
        let bytes = REQUEST.replace("Version: 6", "Version: 5");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::UnsupportedVersion)
        );
    }

    #[test]
    fn test_invalid_start_line() {
        let bytes = REQUEST.replace("GET / HTTP/1.1", "POST / HTTP/1.1");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::InvalidStartLine)
        );
    }

    #[test]
    fn test_id_too_long() {
        let id = "a".repeat(256);
        let bytes = REQUEST.replace("Id: test-client", &format!("Id: {id}"));
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::InvalidID(
                crate::fixed_str::FixedStrError::TooLong { len: 256 }
            ))
        );
    }

    #[test]
    fn test_err() {
        let mut reader = RequestReader::default();
        assert_eq!(
            feed(&mut reader, b"boo\r\n\r\n"),
            Err(RequestReaderError::Parse(httparse::Error::Token))
        );
    }

    fn with_extra_headers(count: usize) -> String {
        let extra: String = (0..count).map(|i| format!("X-Extra-{i}: 1\r\n")).collect();
        REQUEST.replacen("\r\n\r\n", &format!("\r\n{extra}\r\n"), 1)
    }

    #[test]
    fn test_max_headers() {
        let bytes = with_extra_headers(4);
        assert_eq!(read_all(bytes.as_bytes()), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_too_many_headers() {
        let bytes = with_extra_headers(5);
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::Parse(httparse::Error::TooManyHeaders))
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = REQUEST.replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(read_all(bytes.as_bytes()), Ok(Some((req(), Vec::new()))));
    }

    #[test]
    fn test_too_large() {
        let bytes = REQUEST.replacen(
            "\r\n\r\n",
            &format!(
                "\r\nX-Long: {}\r\n\r\n",
                "a".repeat(RequestReader::MAX_REQUEST_SIZE)
            ),
            1,
        );
        let mut reader = RequestReader::default();
        let res = bytes
            .as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|chunk| feed(&mut reader, chunk))
            .find(Result::is_err);
        assert_eq!(res, Some(Err(RequestReaderError::TooLarge)));
    }

    #[test]
    fn test_filled_too_much() {
        let mut reader = RequestReader::default();
        assert_eq!(
            reader.filled(RequestReader::MAX_REQUEST_SIZE + 1),
            Err(RequestReaderError::FilledTooMuch {
                filled: RequestReader::MAX_REQUEST_SIZE + 1,
                remaining: RequestReader::MAX_REQUEST_SIZE,
            })
        );
    }
}
