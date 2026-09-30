use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX, Message, START_LINE,
    TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest, VERSION_PREFIX, Version,
    line_reader::LineReader, prelude::*, strip_prefix_ignore_ascii_case,
};
use anyhow::{Context, Result, anyhow};
use core::num::NonZeroUsize;

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
        buf: [u8; Self::BUFFER_SIZE],
        len: NonZeroUsize,
    ) -> Completion<UpgradeRequest, anyhow::Error, ()> {
        let Some(buf) = buf.get(..len.get()) else {
            return Failed(anyhow!("malformed buffer"));
        };

        for (pos, &byte) in buf.iter().enumerate() {
            let (line, len) = match self.lines.push(byte) {
                Done(line) => line,
                Pending(()) => continue,
                Failed(err) => return Failed(err),
            };
            let Some(line) = line.get(..len) else {
                return Failed(anyhow!("malformed line"));
            };

            let line = match HttpLine::parse(line) {
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
                        return Failed(anyhow!("got leftover in UpgradeRequestReader"));
                    }
                    break;
                }
                HttpLine::Other => {}
            }
        }

        if let Some(req) = self.try_finish() {
            Done(req)
        } else if self.seen_eos {
            Failed(anyhow!("got EOS but no complete UpgradeRequest"))
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
    fn parse(line: &[u8]) -> Result<Self> {
        let line = core::str::from_utf8(line).context("non-utf8 header")?;

        if line == START_LINE {
            Ok(Self::StartLine)
        } else if let Some(host) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX) {
            let host = HostPort::new(host).context("malformed host")?;
            Ok(Self::HostPort(host))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX) {
            let token = Token::new(value).context("malformed token")?;
            Ok(Self::Token(token))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX) {
            let id = ID::new(value).context("malformed id")?;
            Ok(Self::ID(id))
        } else if let Some(version) = strip_prefix_ignore_ascii_case(line, VERSION_PREFIX) {
            let version = Version::new(version).context("malformed version")?;
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

#[cfg(test)]
mod tests {
    use super::UpgradeRequestReader;
    use crate::{
        Completion, HostPort, ID, Token, UpgradeRequest, UpgradeRequestWriter, Version,
        test_helpers::{as_chunks_with_guaranteed_trailer, non_zero_usize},
    };
    use anyhow::{Context, Result};

    fn new_reqwest() -> Result<UpgradeRequest> {
        Ok(UpgradeRequest {
            host: HostPort::new("localhost:3000")?,
            token: Token::new("sekret")?,
            id: ID::new("test-client")?,
            version: Version::new("1.2.3")?,
        })
    }

    #[test]
    fn test_leftover() -> Result<()> {
        let w = UpgradeRequestWriter::new(new_reqwest()?)?;
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer(w.remainder()?);

        let mut reader = UpgradeRequestReader::new();

        for (buf, len) in chunks {
            reader
                .received(buf, len)
                .expect_pending("trailer hasn't been written yet");
        }

        let (mut buf, mut len) = trailer;
        buf.get_mut(len.get()..len.get().checked_add(3).context("bug")?)
            .context("bug")?
            .copy_from_slice(b"abc");
        len = non_zero_usize(len.get() + 3)?;

        let err = reader
            .received(buf, len)
            .expect_failed("there's 'abc' leftover");
        assert_eq!(err.to_string(), "got leftover in UpgradeRequestReader");

        Ok(())
    }

    #[test]
    fn test_no_leftover() -> Result<()> {
        let w = UpgradeRequestWriter::new(new_reqwest()?)?;
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeRequestReader::BUFFER_SIZE },
        >(w.remainder()?);

        let mut reader = UpgradeRequestReader::new();

        for (buf, len) in chunks {
            reader
                .received(buf, len)
                .expect_pending("trailer hasn't been written yet");
        }

        let (buf, len) = trailer;
        let req = reader
            .received(buf, len)
            .expect_done("we've written the trailer");

        assert_eq!(req, new_reqwest()?);

        Ok(())
    }

    #[test]
    fn test_err() -> Result<()> {
        let mut reader = UpgradeRequestReader::new();

        let mut buf = [0; _];
        let malformed = b"boo\r\n\r\n";
        buf.get_mut(..malformed.len())
            .context("bug")?
            .copy_from_slice(malformed);
        let len = non_zero_usize(malformed.len())?;

        let err = reader
            .received(buf, len)
            .expect_failed("incomplete request");

        assert_eq!(err.to_string(), "got EOS but no complete UpgradeRequest");

        Ok(())
    }

    fn with_extra_header(header: &str) -> Result<String> {
        let w = UpgradeRequestWriter::new(new_reqwest()?)?;
        let head = core::str::from_utf8(w.remainder()?)?
            .strip_suffix("\r\n")
            .context("request must end with an empty line")?;
        Ok(format!("{head}{header}\r\n\r\n"))
    }

    fn read_all(bytes: &[u8]) -> Completion<UpgradeRequest, anyhow::Error, ()> {
        let (chunks, trailer) =
            as_chunks_with_guaranteed_trailer::<{ UpgradeRequestReader::BUFFER_SIZE }>(bytes);

        let mut reader = UpgradeRequestReader::new();
        for (buf, len) in chunks {
            reader
                .received(buf, len)
                .expect_pending("trailer hasn't been written yet");
        }
        let (buf, len) = trailer;
        reader.received(buf, len)
    }

    #[test]
    fn test_long_unknown_header() -> Result<()> {
        let bytes = with_extra_header(&format!("X-Long: {}", "a".repeat(300)))?;
        let req = read_all(bytes.as_bytes()).expect_done("long unknown header is skipped");
        assert_eq!(req, new_reqwest()?);
        Ok(())
    }

    #[test]
    fn test_long_known_header() -> Result<()> {
        let w = UpgradeRequestWriter::new(new_reqwest()?)?;
        let bytes = core::str::from_utf8(w.remainder()?)?
            .replace("Token: sekret", &format!("Token: {}", "a".repeat(300)));

        let err = read_all(bytes.as_bytes()).expect_failed("long token line is skipped");
        assert_eq!(err.to_string(), "got EOS but no complete UpgradeRequest");
        Ok(())
    }
}
