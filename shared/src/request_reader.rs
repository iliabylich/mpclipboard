use crate::{ID, NonEmptyStringError, PROTOCOL_VERSION, Request, Token, prelude::*};
use alloc::string::String;
use bytes::BytesMut;
use core::str::Utf8Error;

#[must_use]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RequestReader {
    buf: BytesMut,
}

impl RequestReader {
    const MAX_REQUEST_SIZE: usize = 8192;
    const MAX_HEADERS: usize = 32;

    pub fn received(
        &mut self,
        bytes: &[u8],
    ) -> Result<Completion<(Request, BytesMut), ()>, RequestReaderError> {
        self.buf.extend_from_slice(bytes);

        let mut headers = [httparse::EMPTY_HEADER; Self::MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let httparse::Status::Complete(len) = req.parse(&self.buf)? else {
            if self.buf.len() > Self::MAX_REQUEST_SIZE {
                return Err(RequestReaderError::TooLarge);
            }
            return Ok(Pending(()));
        };

        let request = parse_request(&req)?;
        let leftover = self.buf.split_off(len);
        Ok(Done((request, leftover)))
    }
}

fn parse_request(req: &httparse::Request<'_, '_>) -> Result<Request, RequestReaderError> {
    use RequestReaderError::{
        InvalidID, InvalidStartLine, InvalidToken, MissingHeader, NotChunked, UnsupportedVersion,
    };

    if req.method != Some("POST") || req.path != Some("/") || req.version != Some(1) {
        return Err(InvalidStartLine);
    }

    let mut token = None;
    let mut id = None;
    let mut version = None;
    let mut chunked = false;

    for header in req.headers.iter() {
        let name = header.name;
        let value = || core::str::from_utf8(header.value);

        if name.eq_ignore_ascii_case("Token") {
            token = Some(Token::new(value()?).map_err(InvalidToken)?);
        } else if name.eq_ignore_ascii_case("ID") {
            id = Some(ID::new(value()?).map_err(InvalidID)?);
        } else if name.eq_ignore_ascii_case("Version") {
            version = Some(value()?);
        } else if name.eq_ignore_ascii_case("Transfer-Encoding") {
            chunked = header.value.eq_ignore_ascii_case(b"chunked");
        }
    }

    if !chunked {
        return Err(NotChunked);
    }

    let version = version.ok_or(MissingHeader("Version"))?;
    if version != PROTOCOL_VERSION {
        return Err(UnsupportedVersion(version.into()));
    }

    Ok(Request {
        token: token.ok_or(MissingHeader("Token"))?,
        id: id.ok_or(MissingHeader("ID"))?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequestReaderError {
    #[error("malformed HTTP request: {0}")]
    Parse(#[from] httparse::Error),
    #[error("HTTP request exceeds {} bytes", RequestReader::MAX_REQUEST_SIZE)]
    TooLarge,
    #[error("unexpected start line, expected POST / HTTP/1.1")]
    InvalidStartLine,
    #[error("non-utf8 header: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("malformed token: {0}")]
    InvalidToken(NonEmptyStringError),
    #[error("malformed id: {0}")]
    InvalidID(NonEmptyStringError),
    #[error("unsupported protocol version: given={0}, expected={PROTOCOL_VERSION}")]
    UnsupportedVersion(String),
    #[error("missing {0} header")]
    MissingHeader(&'static str),
    #[error("request is not Transfer-Encoding: chunked")]
    NotChunked,
}

#[cfg(test)]
mod tests {
    use super::{RequestReader, RequestReaderError};
    use crate::{ID, Request, Token, prelude::*};
    use bytes::BytesMut;
    use core::assert_matches;

    const CHUNK_SIZE: usize = 20;

    fn req() -> Request {
        Request {
            token: Token::new("sekret").unwrap(),
            id: ID::new("test-client").unwrap(),
        }
    }

    const REQUEST: &str = "POST / HTTP/1.1\r\n\
Host: localhost:8443\r\n\
User-Agent: mpclipboard\r\n\
Transfer-Encoding: chunked\r\n\
Id: test-client\r\n\
Token: sekret\r\n\
Version: 5\r\n\
Via: 3.0 Caddy\r\n\
X-Forwarded-For: 127.0.0.1\r\n\
X-Forwarded-Host: localhost:8443\r\n\
X-Forwarded-Proto: https\r\n\
Accept-Encoding: gzip\r\n\
\r\n";

    fn read_all(bytes: &[u8]) -> Result<Completion<(Request, BytesMut), ()>, RequestReaderError> {
        let (head, trailer) = bytes.split_at(bytes.len() - CHUNK_SIZE);
        let mut reader = RequestReader::default();
        for chunk in head.chunks(CHUNK_SIZE) {
            assert_eq!(reader.received(chunk), Ok(Pending(())));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(
            read_all(REQUEST.as_bytes()),
            Ok(Done((req(), BytesMut::new())))
        );
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{REQUEST}abc");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Ok(Done((req(), BytesMut::from(&b"abc"[..]))))
        );
    }

    #[test]
    fn test_not_chunked() {
        let bytes = REQUEST.replace("Transfer-Encoding: chunked\r\n", "Content-Length: 5\r\n");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::NotChunked)
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
        let bytes = REQUEST.replace("Version: 5", "Version: 4");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::UnsupportedVersion("4".to_string()))
        );
    }

    #[test]
    fn test_invalid_start_line() {
        let bytes = REQUEST.replace("POST / HTTP/1.1", "GET / HTTP/1.1");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::InvalidStartLine)
        );
    }

    #[test]
    fn test_err() {
        let mut reader = RequestReader::default();
        assert_matches!(
            reader.received(b"boo\r\n\r\n"),
            Err(RequestReaderError::Parse(_))
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = REQUEST.replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(
            read_all(bytes.as_bytes()),
            Ok(Done((req(), BytesMut::new())))
        );
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
            .map(|chunk| reader.received(chunk))
            .find(Result::is_err);
        assert_eq!(res, Some(Err(RequestReaderError::TooLarge)));
    }
}
