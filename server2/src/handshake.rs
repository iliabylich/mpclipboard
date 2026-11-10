use crate::{
    ServerContext,
    fixed_str::{FixedStr, FixedStrError},
};
use core::{net::SocketAddr, str::Utf8Error};
use externally_driven_async_rt::net::TcpStream;

pub(crate) type Token = FixedStr<20>;
pub(crate) type ID = FixedStr<20>;

const MAX_REQUEST_SIZE: usize = 2048;
const MAX_HEADERS: usize = 32;
const PROTOCOL_VERSION: &str = "6";
const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

pub(crate) struct Handshake {
    buf: [u8; MAX_REQUEST_SIZE],
    len: usize,
}

impl Handshake {
    pub(crate) async fn read(
        &mut self,
        ctx: &ServerContext<'_>,
        stream: &mut TcpStream,
        peer: SocketAddr,
    ) -> Result<(Token, ID, &[u8]), ()> {
        loop {
            self.len += match stream.read(ctx, &mut self.buf[self.len..]).await {
                Ok(0) => {
                    log::info!("[{peer}] disconnected during handshake");
                    return Err(());
                }
                Ok(read) => read,
                Err(err) => {
                    log::error!("[{peer}] read() failed during handshake: {err}");
                    return Err(());
                }
            };

            match parse(&self.buf[..self.len]) {
                Ok(None) => {}
                Ok(Some((token, id, consumed))) => {
                    return Ok((token, id, &self.buf[consumed..self.len]));
                }
                Err(err) => {
                    log::error!("[{peer}] {err}");
                    return Err(());
                }
            }
        }
    }
}

impl Default for Handshake {
    fn default() -> Self {
        Self {
            buf: [0; MAX_REQUEST_SIZE],
            len: 0,
        }
    }
}

fn parse(bytes: &[u8]) -> Result<Option<(Token, ID, usize)>, Error> {
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut req = httparse::Request::new(&mut headers);
    let httparse::Status::Complete(len) = req.parse(bytes)? else {
        if bytes.len() >= MAX_REQUEST_SIZE {
            return Err(Error::TooLarge);
        }
        return Ok(None);
    };

    let (token, id) = parse_request(&req)?;
    Ok(Some((token, id, len)))
}

fn parse_request(req: &httparse::Request<'_, '_>) -> Result<(Token, ID), Error> {
    use Error::{
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
            version = Some(header.value);
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
    if version != PROTOCOL_VERSION.as_bytes() {
        return Err(UnsupportedVersion);
    }

    Ok((
        token.ok_or(MissingHeader("Token"))?,
        id.ok_or(MissingHeader("ID"))?,
    ))
}

fn has_token(value: &[u8], token: &str) -> bool {
    value
        .split(|byte| *byte == b',')
        .any(|item| item.trim_ascii().eq_ignore_ascii_case(token.as_bytes()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Error {
    Parse(httparse::Error),
    TooLarge,
    InvalidStartLine,
    NonUtf8(Utf8Error),
    InvalidToken(FixedStrError),
    InvalidID(FixedStrError),
    NotUpgrade,
    UnsupportedUpgrade,
    UnsupportedVersion,
    MissingHeader(&'static str),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "malformed HTTP request: {err}"),
            Self::TooLarge => write!(f, "HTTP request exceeds {MAX_REQUEST_SIZE} bytes"),
            Self::InvalidStartLine => f.write_str("unexpected start line, expected GET / HTTP/1.1"),
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

impl From<httparse::Error> for Error {
    fn from(err: httparse::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::NonUtf8(err)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, ID, MAX_REQUEST_SIZE, Token, parse};
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

    type Parsed = Result<Option<(Token, ID, Vec<u8>)>, Error>;

    fn parsed(leftover: &[u8]) -> Parsed {
        Ok(Some((
            Token::new("sekret").unwrap(),
            ID::new("test-client").unwrap(),
            leftover.to_vec(),
        )))
    }

    fn read_in_chunks(bytes: &[u8], chunk_size: usize) -> Parsed {
        let mut len = 0;
        loop {
            let read = chunk_size
                .min(MAX_REQUEST_SIZE - len)
                .min(bytes.len() - len);
            if read == 0 {
                return Ok(None);
            }
            len += read;
            if let Some((token, id, consumed)) = parse(&bytes[..len])? {
                return Ok(Some((token, id, bytes[consumed..].to_vec())));
            }
        }
    }

    fn read_all(bytes: &str) -> Parsed {
        read_all_bytes(bytes.as_bytes())
    }

    fn read_all_bytes(bytes: &[u8]) -> Parsed {
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
        assert_eq!(read_all(REQUEST), parsed(b""));
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{REQUEST}abc");
        assert_eq!(read_all(&bytes), parsed(b"abc"));
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(read_all(&REQUEST[..REQUEST.len() - 2]), Ok(None));
    }

    #[test]
    fn test_connection_list() {
        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: close, upgrade");
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    #[test]
    fn test_not_upgrade() {
        let bytes = REQUEST.replace("Connection: Upgrade\r\n", "");
        assert_eq!(read_all(&bytes), Err(Error::NotUpgrade));

        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive");
        assert_eq!(read_all(&bytes), Err(Error::NotUpgrade));
    }

    #[test]
    fn test_unsupported_upgrade() {
        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw\r\n", "");
        assert_eq!(read_all(&bytes), Err(Error::UnsupportedUpgrade));

        let bytes = REQUEST.replace("Upgrade: mpclipboard-raw", "Upgrade: websocket");
        assert_eq!(read_all(&bytes), Err(Error::UnsupportedUpgrade));
    }

    #[test]
    fn test_missing_header() {
        let bytes = REQUEST.replace("Id: test-client\r\n", "");
        assert_eq!(read_all(&bytes), Err(Error::MissingHeader("ID")));

        let bytes = REQUEST.replace("Version: 6\r\n", "");
        assert_eq!(read_all(&bytes), Err(Error::MissingHeader("Version")));
    }

    #[test]
    fn test_unsupported_version() {
        let bytes = REQUEST.replace("Version: 6", "Version: 5");
        assert_eq!(read_all(&bytes), Err(Error::UnsupportedVersion));
    }

    #[test]
    fn test_invalid_start_line() {
        let bytes = REQUEST.replace("GET / HTTP/1.1", "POST / HTTP/1.1");
        assert_eq!(read_all(&bytes), Err(Error::InvalidStartLine));
    }

    #[test]
    fn test_too_large() {
        let bytes = with_header(&format!("X-Long: {}", "a".repeat(MAX_REQUEST_SIZE)));
        assert_eq!(read_all(&bytes), Err(Error::TooLarge));
    }

    #[test]
    fn test_leftover_frames() {
        let frames = [1_u8; 3 * 256];
        let mut bytes = REQUEST.as_bytes().to_vec();
        bytes.extend_from_slice(&frames);
        assert_eq!(read_all_bytes(&bytes), parsed(&frames));
    }

    #[test]
    fn test_token_max_len() {
        let token = "a".repeat(20);
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {token}"));
        let parsed = read_all(&bytes).unwrap().unwrap();
        assert_eq!(parsed.0, Token::new(&token).unwrap());
    }

    #[test]
    fn test_token_too_long() {
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {}", "a".repeat(23)));
        assert_eq!(
            read_all(&bytes),
            Err(Error::InvalidToken(FixedStrError::TooLong { len: 23 }))
        );
    }

    #[test]
    fn test_long_connection_list() {
        let bytes = REQUEST.replace("Connection: Upgrade", "Connection: keep-alive, Upgrade");
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    #[test]
    fn test_long_unused_header_is_skipped() {
        let bytes = with_header(&format!("X-Long: {}", "a".repeat(300)));
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    #[test]
    fn test_long_unused_header_name_is_skipped() {
        let bytes = with_header(&format!("X-{}: 1", "a".repeat(100)));
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    #[test]
    fn test_used_header_after_skipped_one() {
        let bytes = REQUEST.replace("Token: sekret\r\n", "").replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\nToken: sekret\r\n\r\n", "a".repeat(100)),
            1,
        );
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    #[test]
    fn test_garbage_start_line() {
        assert_eq!(
            read_all("boo\r\n\r\n"),
            Err(Error::Parse(httparse::Error::Token))
        );
    }

    #[test]
    fn test_malformed_header() {
        assert_eq!(
            read_all(&with_header("no colon")),
            Err(Error::Parse(httparse::Error::HeaderName))
        );
        assert_eq!(
            read_all(&with_header(": empty name")),
            Err(Error::Parse(httparse::Error::HeaderName))
        );
        assert_eq!(
            read_all(&with_header("Token : sekret")),
            Err(Error::Parse(httparse::Error::HeaderName))
        );
    }

    #[test]
    fn test_header_value_whitespace() {
        let bytes = REQUEST.replace("Token: sekret", "Token:\t sekret  ");
        assert_eq!(read_all(&bytes), parsed(b""));

        let bytes = REQUEST.replace("Token: sekret", "Token:sekret");
        assert_eq!(read_all(&bytes), parsed(b""));
    }

    fn with_extra_headers(count: usize) -> String {
        let extra: String = (0..count).map(|i| format!("X-Extra-{i}: 1\r\n")).collect();
        REQUEST.replacen("\r\n\r\n", &format!("\r\n{extra}\r\n"), 1)
    }

    #[test]
    fn test_max_headers() {
        assert_eq!(read_all(&with_extra_headers(20)), parsed(b""));
    }

    #[test]
    fn test_too_many_headers() {
        assert_eq!(
            read_all(&with_extra_headers(21)),
            Err(Error::Parse(httparse::Error::TooManyHeaders))
        );
    }
}
