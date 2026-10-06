use crate::message::MessageSize;
use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX,
    NonEmptyInlineStringError, START_LINE, TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    UpgradeRequest, VERSION_PREFIX, Version,
    line_reader::{LineReader, LineReaderError},
    prelude::*,
    strip_prefix_ignore_ascii_case,
};
use core::{
    ops::ControlFlow::{Break, Continue},
    str::Utf8Error,
};
use typenum::{U1, op};

type BufferSize = op!(MessageSize - U1);

#[expect(clippy::struct_excessive_bools)]
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpgradeRequestReader {
    lines: LineReader<BufferSize>,

    seen_start_line: bool,

    host: Option<HostPort>,
    token: Option<Token>,
    id: Option<ID>,
    version: Option<Version>,

    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
    seen_eos: bool,
}

impl UpgradeRequestReader {
    pub fn new() -> Self {
        Self {
            lines: LineReader::new(),

            seen_start_line: false,

            host: None,
            token: None,
            id: None,
            version: None,

            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
            seen_eos: false,
        }
    }

    pub fn received(
        &mut self,
        mut buf: Buffer<BufferSize>,
    ) -> Result<Completion<UpgradeRequest, ()>, UpgradeRequestReaderError> {
        buf.drain_front::<UpgradeRequestReaderError>(|byte| {
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
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfRequest => {
                    self.seen_eos = true;
                    return Ok(Break(()));
                }
                HttpLine::Other => {}
            }

            Ok(Continue(()))
        })?;

        if !buf.as_slice().is_empty() {
            return Err(UpgradeRequestReaderError::Leftover);
        }

        if let Some(req) = self.try_finish() {
            Ok(Done(req))
        } else if self.seen_eos {
            Err(UpgradeRequestReaderError::Incomplete)
        } else {
            Ok(Pending(()))
        }
    }

    fn try_finish(&self) -> Option<UpgradeRequest> {
        if self.seen_start_line
            && let Some(host) = &self.host
            && let Some(token) = &self.token
            && let Some(id) = &self.id
            && let Some(version) = &self.version
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
            && self.seen_eos
        {
            Some(UpgradeRequest {
                host: host.clone(),
                token: token.clone(),
                id: id.clone(),
                version: version.clone(),
            })
        } else {
            None
        }
    }
}

impl Default for UpgradeRequestReader {
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
    ConnectionUpgrade,
    UpgradeMPClipboardRaw,
    EndOfRequest,

    Other,
}

impl HttpLine {
    fn parse(line: &[u8]) -> Result<Self, UpgradeRequestReaderError> {
        use UpgradeRequestReaderError::{InvalidHost, InvalidID, InvalidToken, InvalidVersion};

        let line = core::str::from_utf8(line)?;

        if line.as_bytes() == START_LINE.as_slice() {
            Ok(Self::StartLine)
        } else if let Some(host) = strip_prefix_ignore_ascii_case(line, &HOST_PREFIX) {
            let host = HostPort::new(host).map_err(InvalidHost)?;
            Ok(Self::HostPort(host))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, &TOKEN_PREFIX) {
            let token = Token::new(value).map_err(InvalidToken)?;
            Ok(Self::Token(token))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, &ID_PREFIX) {
            let id = ID::new(value).map_err(InvalidID)?;
            Ok(Self::ID(id))
        } else if let Some(version) = strip_prefix_ignore_ascii_case(line, &VERSION_PREFIX) {
            let version = Version::new(version).map_err(InvalidVersion)?;
            Ok(Self::Version(version))
        } else if strip_prefix_ignore_ascii_case(line, &CONNECTION_UPGRADE_HEADER) == Some("") {
            Ok(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(line, &UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("")
        {
            Ok(Self::UpgradeMPClipboardRaw)
        } else if line.is_empty() {
            Ok(Self::EndOfRequest)
        } else {
            Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UpgradeRequestReaderError {
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
    #[error("got leftover in UpgradeRequestReader")]
    Leftover,
    #[error("got EOS but no complete UpgradeRequest")]
    Incomplete,
}

#[cfg(test)]
mod tests {
    use super::{UpgradeRequestReader, UpgradeRequestReaderError};
    use crate::{
        HostPort, ID, Token, UpgradeRequest, UpgradeRequestWriter, Version,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
    };
    use alloc::{format, string::String};

    const REQ: UpgradeRequest = UpgradeRequest {
        host: HostPort::const_new("localhost:3000"),
        token: Token::const_new("sekret"),
        id: ID::const_new("test-client"),
        version: Version::const_new("1.2.3"),
    };

    fn request() -> String {
        String::from_utf8_lossy(UpgradeRequestWriter::new(&REQ).remainder()).into_owned()
    }

    fn read_all(bytes: &[u8]) -> Result<Completion<UpgradeRequest, ()>, UpgradeRequestReaderError> {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<super::BufferSize>(bytes);

        let mut reader = UpgradeRequestReader::new();
        for buf in chunks {
            assert_eq!(reader.received(buf), Ok(Pending(())));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(request().as_bytes()), Ok(Done(REQ)));
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{}abc", request());
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(UpgradeRequestReaderError::Leftover)
        );
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeRequestReader::new();
        assert_eq!(
            reader.received(buffer(b"boo\r\n\r\n")),
            Err(UpgradeRequestReaderError::Incomplete)
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = request().replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(read_all(bytes.as_bytes()), Ok(Done(REQ)));
    }

    #[test]
    fn test_long_known_header() {
        let bytes = request().replace("Token: sekret", &format!("Token: {}", "a".repeat(300)));
        assert_eq!(
            read_all(bytes.as_bytes()),
            Err(UpgradeRequestReaderError::Incomplete)
        );
    }
}
