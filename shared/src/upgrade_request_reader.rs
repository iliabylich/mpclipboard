use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX, Message,
    NonEmptyInlineStringError, START_LINE, TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    UpgradeRequest, VERSION_PREFIX, Version,
    line_reader::{LineReader, LineReaderError},
    prelude::*,
    strip_prefix_ignore_ascii_case,
};
use core::str::Utf8Error;

#[expect(clippy::struct_excessive_bools)]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpgradeRequestReader {
    lines: LineReader<{ Self::BUFFER_SIZE }>,

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
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;

    pub const fn new() -> Self {
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
        buf: Buffer<{ Self::BUFFER_SIZE }>,
    ) -> Completion<UpgradeRequest, UpgradeRequestReaderError, ()> {
        let buf = buf.as_slice();

        for (pos, &byte) in buf.iter().enumerate() {
            let line = match self.lines.push(byte) {
                Done(line) => line,
                Pending(()) => continue,
                Failed(err) => return Failed(UpgradeRequestReaderError::Line(err)),
            };

            let line = match HttpLine::parse(line.as_slice()) {
                Ok(line) => line,
                Err(err) => return Failed(err),
            };

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

                    if pos.checked_add(1) != Some(buf.len()) {
                        return Failed(UpgradeRequestReaderError::Leftover);
                    }
                    break;
                }
                HttpLine::Other => {}
            }
        }

        if let Some(req) = self.try_finish() {
            Done(req)
        } else if self.seen_eos {
            Failed(UpgradeRequestReaderError::Incomplete)
        } else {
            Pending(())
        }
    }

    const fn try_finish(&self) -> Option<UpgradeRequest> {
        if self.seen_start_line
            && let Some(host) = self.host
            && let Some(token) = self.token
            && let Some(id) = self.id
            && let Some(version) = self.version
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
            && self.seen_eos
        {
            Some(UpgradeRequest {
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
        use UpgradeRequestReaderError::{
            InvalidHost, InvalidID, InvalidToken, InvalidVersion, NonUtf8,
        };

        let line = core::str::from_utf8(line).map_err(NonUtf8)?;

        if line == START_LINE {
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
        } else if strip_prefix_ignore_ascii_case(line, CONNECTION_UPGRADE_HEADER) == Some("") {
            Ok(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(line, UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("") {
            Ok(Self::UpgradeMPClipboardRaw)
        } else if line.is_empty() {
            Ok(Self::EndOfRequest)
        } else {
            Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeRequestReaderError {
    Line(LineReaderError),
    NonUtf8(Utf8Error),
    InvalidHost(NonEmptyInlineStringError),
    InvalidToken(NonEmptyInlineStringError),
    InvalidID(NonEmptyInlineStringError),
    InvalidVersion(NonEmptyInlineStringError),
    Leftover,
    Incomplete,
}

impl core::fmt::Display for UpgradeRequestReaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Line(err) => write!(f, "{err}"),
            Self::NonUtf8(err) => write!(f, "non-utf8 header: {err}"),
            Self::InvalidHost(err) => write!(f, "malformed host: {err}"),
            Self::InvalidToken(err) => write!(f, "malformed token: {err}"),
            Self::InvalidID(err) => write!(f, "malformed id: {err}"),
            Self::InvalidVersion(err) => write!(f, "malformed version: {err}"),
            Self::Leftover => write!(f, "got leftover in UpgradeRequestReader"),
            Self::Incomplete => write!(f, "got EOS but no complete UpgradeRequest"),
        }
    }
}

impl core::error::Error for UpgradeRequestReaderError {}

#[cfg(test)]
mod tests {
    use super::{UpgradeRequestReader, UpgradeRequestReaderError};
    use crate::{
        HostPort, ID, Token, UpgradeRequest, UpgradeRequestWriter, Version,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
    };

    const REQ: UpgradeRequest = UpgradeRequest {
        host: HostPort::const_new("localhost:3000"),
        token: Token::const_new("sekret"),
        id: ID::const_new("test-client"),
        version: Version::const_new("1.2.3"),
    };

    fn request() -> String {
        String::from_utf8_lossy(UpgradeRequestWriter::new(REQ).remainder()).into_owned()
    }

    fn read_all(bytes: &[u8]) -> Completion<UpgradeRequest, UpgradeRequestReaderError, ()> {
        let (chunks, trailer) =
            as_chunks_with_guaranteed_trailer::<{ UpgradeRequestReader::BUFFER_SIZE }>(bytes);

        let mut reader = UpgradeRequestReader::new();
        for buf in chunks {
            assert_eq!(reader.received(buf), Pending(()));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(request().as_bytes()), Done(REQ));
    }

    #[test]
    fn test_leftover() {
        let bytes = format!("{}abc", request());
        assert_eq!(
            read_all(bytes.as_bytes()),
            Failed(UpgradeRequestReaderError::Leftover)
        );
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeRequestReader::new();
        assert_eq!(
            reader.received(buffer(b"boo\r\n\r\n")),
            Failed(UpgradeRequestReaderError::Incomplete)
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = request().replacen(
            "\r\n\r\n",
            &format!("\r\nX-Long: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(read_all(bytes.as_bytes()), Done(REQ));
    }

    #[test]
    fn test_long_known_header() {
        let bytes = request().replace("Token: sekret", &format!("Token: {}", "a".repeat(300)));
        assert_eq!(
            read_all(bytes.as_bytes()),
            Failed(UpgradeRequestReaderError::Incomplete)
        );
    }
}
