use crate::{
    Buffer, HOST_PREFIX, HostPort, ID, ID_PREFIX, Message, NonEmptyInlineStringError, Request,
    START_LINE, TOKEN_PREFIX, TRANSFER_ENCODING_CHUNKED_HEADER, Token, VERSION_PREFIX, Version,
    line_reader::{LineReader, LineReaderError},
    prelude::*,
    strip_prefix_ignore_ascii_case,
};
use core::{
    ops::ControlFlow::{Break, Continue},
    str::Utf8Error,
};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestReader {
    lines: LineReader<{ Self::BUFFER_SIZE }>,

    seen_start_line: bool,

    host: Option<HostPort>,
    token: Option<Token>,
    id: Option<ID>,
    version: Option<Version>,

    seen_transfer_encoding_chunked: bool,
    seen_eos: bool,
}

impl RequestReader {
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;

    pub const fn new() -> Self {
        Self {
            lines: LineReader::new(),

            seen_start_line: false,

            host: None,
            token: None,
            id: None,
            version: None,

            seen_transfer_encoding_chunked: false,
            seen_eos: false,
        }
    }

    pub fn received(
        &mut self,
        mut buf: Buffer<{ Self::BUFFER_SIZE }>,
    ) -> Result<Completion<(Request, Buffer<{ Self::BUFFER_SIZE }>), ()>, RequestReaderError> {
        buf.drain_front::<RequestReaderError>(|byte| {
            let Done(line) = self.lines.push(byte)? else {
                return Ok(Continue(()));
            };

            let line = HttpLine::parse(line.as_slice())?;

            match line {
                HttpLine::StartLine => self.seen_start_line = true,
                HttpLine::HostPort(host) => self.host = Some(host),
                HttpLine::Token(token) => self.token = Some(token),
                HttpLine::ID(id) => self.id = Some(id),
                HttpLine::Version(version) => self.version = Some(version),
                HttpLine::TransferEncodingChunked => self.seen_transfer_encoding_chunked = true,
                HttpLine::EndOfRequest => {
                    self.seen_eos = true;
                    return Ok(Break(()));
                }
                HttpLine::Other => {}
            }

            Ok(Continue(()))
        })?;

        if let Some(req) = self.try_finish() {
            Ok(Done((req, buf)))
        } else if self.seen_eos {
            Err(RequestReaderError::Incomplete)
        } else {
            Ok(Pending(()))
        }
    }

    const fn try_finish(&self) -> Option<Request> {
        if self.seen_start_line
            && let Some(host) = self.host
            && let Some(token) = self.token
            && let Some(id) = self.id
            && let Some(version) = self.version
            && self.seen_transfer_encoding_chunked
            && self.seen_eos
        {
            Some(Request {
                host,
                token,
                id,
                version,
            })
        } else {
            None
        }
    }
}

impl Default for RequestReader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum HttpLine {
    StartLine,
    HostPort(HostPort),
    Token(Token),
    ID(ID),
    Version(Version),
    TransferEncodingChunked,
    EndOfRequest,

    Other,
}

impl HttpLine {
    fn parse(line: &[u8]) -> Result<Self, RequestReaderError> {
        use RequestReaderError::{InvalidHost, InvalidID, InvalidToken, InvalidVersion};

        let line = core::str::from_utf8(line)?;

        if line.as_bytes() == START_LINE {
            Ok(Self::StartLine)
        } else if let Some(host) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX) {
            let host = HostPort::new(host).map_err(InvalidHost)?;
            Ok(Self::HostPort(host))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX) {
            let token = Token::new(value).map_err(InvalidToken)?;
            Ok(Self::Token(token))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX) {
            let id = ID::new(value).map_err(InvalidID)?;
            Ok(Self::ID(id))
        } else if let Some(version) = strip_prefix_ignore_ascii_case(line, VERSION_PREFIX) {
            let version = Version::new(version).map_err(InvalidVersion)?;
            Ok(Self::Version(version))
        } else if strip_prefix_ignore_ascii_case(line, TRANSFER_ENCODING_CHUNKED_HEADER) == Some("")
        {
            Ok(Self::TransferEncodingChunked)
        } else if line.is_empty() {
            Ok(Self::EndOfRequest)
        } else {
            Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RequestReaderError {
    #[error("{0}")]
    Line(#[from] LineReaderError),
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
    #[error("got EOS but no complete Request")]
    Incomplete,
}

#[cfg(test)]
mod tests {
    use super::{RequestReader, RequestReaderError};
    use crate::{
        Buffer, HostPort, ID, Request, Token, Version,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
    };

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
            Err(RequestReaderError::Incomplete)
        );
    }

    #[test]
    fn test_err() {
        let mut reader = RequestReader::new();
        assert_eq!(
            reader.received(buffer(b"boo\r\n\r\n")),
            Err(RequestReaderError::Incomplete)
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
            Err(RequestReaderError::Incomplete)
        );
    }
}
