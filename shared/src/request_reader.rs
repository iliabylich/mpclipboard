use crate::{
    Buffer, HostPort, ID, Message, NonEmptyInlineStringError, PushResult, Request, Token, Version,
    prelude::*,
};
use core::str::Utf8Error;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestReader {
    buf: Buffer<{ Self::MAX_REQUEST_SIZE }>,
}

impl RequestReader {
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;
    const MAX_REQUEST_SIZE: usize = 8192;
    const MAX_HEADERS: usize = 32;

    pub const fn new() -> Self {
        Self {
            buf: Buffer::empty(),
        }
    }

    pub fn received(
        &mut self,
        buf: Buffer<{ Self::BUFFER_SIZE }>,
    ) -> Result<Completion<(Request, Buffer<{ Self::BUFFER_SIZE }>), ()>, RequestReaderError> {
        for &byte in buf.as_slice() {
            if let PushResult::Full(_) = self.buf.push(byte) {
                return Err(RequestReaderError::TooLarge);
            }
        }

        let mut headers = [httparse::EMPTY_HEADER; Self::MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let httparse::Status::Complete(len) = req.parse(self.buf.as_slice())? else {
            return Ok(Pending(()));
        };

        let request = parse_request(&req);
        let leftover = self.buf.as_slice().get(len..).and_then(Buffer::from_slice);
        self.buf.clear();

        let request = request?;
        let leftover = leftover.unwrap_or_else(|| {
            unreachable!("headers end within the last received chunk, which fits into BUFFER_SIZE")
        });
        Ok(Done((request, leftover)))
    }
}

impl Default for RequestReader {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_request(req: &httparse::Request<'_, '_>) -> Result<Request, RequestReaderError> {
    use RequestReaderError::{
        InvalidHost, InvalidID, InvalidStartLine, InvalidToken, InvalidVersion, MissingHeader,
        NotChunked,
    };

    if req.method != Some("POST") || req.path != Some("/") || req.version != Some(1) {
        return Err(InvalidStartLine);
    }

    let mut host = None;
    let mut token = None;
    let mut id = None;
    let mut version = None;
    let mut chunked = false;

    for header in req.headers.iter() {
        let name = header.name;
        let value = || core::str::from_utf8(header.value);

        if name.eq_ignore_ascii_case("Host") {
            host = Some(HostPort::new(value()?).map_err(InvalidHost)?);
        } else if name.eq_ignore_ascii_case("Token") {
            token = Some(Token::new(value()?).map_err(InvalidToken)?);
        } else if name.eq_ignore_ascii_case("ID") {
            id = Some(ID::new(value()?).map_err(InvalidID)?);
        } else if name.eq_ignore_ascii_case("Version") {
            version = Some(Version::new(value()?).map_err(InvalidVersion)?);
        } else if name.eq_ignore_ascii_case("Transfer-Encoding") {
            chunked = header.value.eq_ignore_ascii_case(b"chunked");
        }
    }

    if !chunked {
        return Err(NotChunked);
    }

    Ok(Request {
        host: host.ok_or(MissingHeader("Host"))?,
        token: token.ok_or(MissingHeader("Token"))?,
        id: id.ok_or(MissingHeader("ID"))?,
        version: version.ok_or(MissingHeader("Version"))?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RequestReaderError {
    #[error("malformed HTTP request: {0}")]
    Parse(#[from] httparse::Error),
    #[error("HTTP request exceeds {} bytes", RequestReader::MAX_REQUEST_SIZE)]
    TooLarge,
    #[error("unexpected start line, expected POST / HTTP/1.1")]
    InvalidStartLine,
    #[error("non-utf8 header: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("malformed host: {0}")]
    InvalidHost(NonEmptyInlineStringError),
    #[error("malformed token: {0}")]
    InvalidToken(NonEmptyInlineStringError),
    #[error("malformed id: {0}")]
    InvalidID(NonEmptyInlineStringError),
    #[error("malformed version: {0}")]
    InvalidVersion(NonEmptyInlineStringError),
    #[error("missing {0} header")]
    MissingHeader(&'static str),
    #[error("request is not Transfer-Encoding: chunked")]
    NotChunked,
}

#[cfg(test)]
mod tests {
    use super::{RequestReader, RequestReaderError};
    use crate::{
        Buffer, HostPort, ID, NonEmptyInlineStringError, Request, Token, Version,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
    };
    use core::assert_matches;

    const REQ: Request = Request {
        host: HostPort::const_new("localhost:8443"),
        token: Token::const_new("sekret"),
        id: ID::const_new("test-client"),
        version: Version::const_new("1.2.3"),
    };

    const REQUEST: &str = "POST / HTTP/1.1\r\n\
Host: localhost:8443\r\n\
User-Agent: mpclipboard\r\n\
Transfer-Encoding: chunked\r\n\
Id: test-client\r\n\
Token: sekret\r\n\
Version: 1.2.3\r\n\
Via: 3.0 Caddy\r\n\
X-Forwarded-For: 127.0.0.1\r\n\
X-Forwarded-Host: localhost:8443\r\n\
X-Forwarded-Proto: https\r\n\
Accept-Encoding: gzip\r\n\
\r\n";

    fn read_all(
        bytes: &[u8],
    ) -> Result<Completion<(Request, Buffer<{ RequestReader::BUFFER_SIZE }>), ()>, RequestReaderError>
    {
        let (chunks, trailer) =
            as_chunks_with_guaranteed_trailer::<{ RequestReader::BUFFER_SIZE }>(bytes);
        let mut reader = RequestReader::new();
        for buf in chunks {
            assert_eq!(reader.received(buf), Ok(Pending(())));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(
            read_all(REQUEST.as_bytes()),
            Ok(Done((REQ, Buffer::empty())))
        );
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{REQUEST}abc");
        assert_eq!(read_all(bytes.as_bytes()), Ok(Done((REQ, buffer(b"abc")))));
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
    fn test_invalid_start_line() {
        let bytes = REQUEST.replace("POST / HTTP/1.1", "GET / HTTP/1.1");
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::InvalidStartLine)
        );
    }

    #[test]
    fn test_err() {
        let mut reader = RequestReader::new();
        assert_matches!(
            reader.received(buffer(b"boo\r\n\r\n")),
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
        assert_eq!(read_all(bytes.as_bytes()), Ok(Done((REQ, Buffer::empty()))));
    }

    #[test]
    fn test_long_known_header() {
        let bytes = REQUEST.replace("Token: sekret", &format!("Token: {}", "a".repeat(300)));
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(RequestReaderError::InvalidToken(
                NonEmptyInlineStringError::TooLong
            ))
        );
    }

    #[test]
    fn test_too_large() {
        let bytes = REQUEST.replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\n\r\n", "a".repeat(RequestReader::MAX_REQUEST_SIZE)),
            1,
        );
        let (chunks, _trailer) =
            as_chunks_with_guaranteed_trailer::<{ RequestReader::BUFFER_SIZE }>(bytes.as_bytes());
        let mut reader = RequestReader::new();
        let res = chunks
            .map(|buf| reader.received(buf))
            .find(Result::is_err);
        assert_eq!(res, Some(Err(RequestReaderError::TooLarge)));
    }
}
