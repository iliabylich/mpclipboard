use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    line_reader::{LineReader, LineReaderError},
    message::Message,
    prelude::*,
    strip_prefix_ignore_ascii_case,
};
use core::str::Utf8Error;

#[expect(clippy::struct_excessive_bools)]
#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseReader {
    lines: LineReader<{ Self::BUFFER_SIZE }>,

    seen_start_line: bool,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
    seen_eos: bool,
}

impl UpgradeResponseReader {
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;

    pub const fn new() -> Self {
        Self {
            lines: LineReader::new(),

            seen_start_line: false,
            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
            seen_eos: false,
        }
    }

    pub fn received(
        &mut self,
        buf: Buffer<{ Self::BUFFER_SIZE }>,
    ) -> Completion<Buffer<{ Self::BUFFER_SIZE }>, UpgradeResponseReaderError, ()> {
        let buf = buf.as_slice();

        let mut leftover = Buffer::new();

        for (pos, &byte) in buf.iter().enumerate() {
            let line = match self.lines.push(byte) {
                Done(line) => line,
                Pending(()) => continue,
                Failed(err) => return Failed(UpgradeResponseReaderError::Line(err)),
            };

            let line = match HttpLine::parse(line.as_slice()) {
                Ok(line) => line,
                Err(err) => return Failed(err),
            };

            match line {
                HttpLine::StartLine => self.seen_start_line = true,
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfResponse => {
                    self.seen_eos = true;

                    let Some(rest) = pos.checked_add(1).and_then(|start| buf.get(start..)) else {
                        unreachable!("pos is an index into buf");
                    };
                    let Some(rest) = Buffer::from_slice(rest) else {
                        unreachable!("rest is a part of a buffer of the same size");
                    };
                    leftover = rest;
                    break;
                }
                HttpLine::Other => {}
            }
        }

        if self.try_finish() {
            Done(leftover)
        } else if self.seen_eos {
            Failed(UpgradeResponseReaderError::Incomplete)
        } else {
            Pending(())
        }
    }

    const fn try_finish(&self) -> bool {
        self.seen_start_line
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
            && self.seen_eos
    }
}

impl Default for UpgradeResponseReader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum HttpLine {
    StartLine,
    ConnectionUpgrade,
    UpgradeMPClipboardRaw,
    EndOfResponse,

    Other,
}

impl HttpLine {
    fn parse(line: &[u8]) -> Result<Self, UpgradeResponseReaderError> {
        let line = core::str::from_utf8(line).map_err(UpgradeResponseReaderError::NonUtf8)?;

        if line == "HTTP/1.1 101 Switching Protocols" {
            Ok(Self::StartLine)
        } else if strip_prefix_ignore_ascii_case(line, CONNECTION_UPGRADE_HEADER) == Some("") {
            Ok(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(line, UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("") {
            Ok(Self::UpgradeMPClipboardRaw)
        } else if line.is_empty() {
            Ok(Self::EndOfResponse)
        } else {
            Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeResponseReaderError {
    Line(LineReaderError),
    NonUtf8(Utf8Error),
    Incomplete,
}

impl core::fmt::Display for UpgradeResponseReaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Line(err) => write!(f, "{err}"),
            Self::NonUtf8(err) => write!(f, "non-utf8 header: {err}"),
            Self::Incomplete => write!(f, "got EOS but UpgradeResponse is incomplete"),
        }
    }
}

impl core::error::Error for UpgradeResponseReaderError {}

#[cfg(test)]
mod tests {
    use super::{UpgradeResponseReader, UpgradeResponseReaderError};
    use crate::{
        Buffer,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
        upgrade_response::UpgradeResponse,
    };

    fn read_all(
        bytes: &[u8],
    ) -> Completion<Buffer<{ UpgradeResponseReader::BUFFER_SIZE }>, UpgradeResponseReaderError, ()>
    {
        let (chunks, trailer) =
            as_chunks_with_guaranteed_trailer::<{ UpgradeResponseReader::BUFFER_SIZE }>(bytes);

        let mut reader = UpgradeResponseReader::new();
        for buf in chunks {
            assert_eq!(reader.received(buf), Pending(()));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(UpgradeResponse::BYTES), Done(Buffer::new()));
    }

    #[test]
    fn test_leftover() {
        let bytes = [UpgradeResponse::BYTES, b"abc"].concat();
        assert_eq!(read_all(&bytes), Done(buffer(b"abc")));
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeResponseReader::new();
        assert_eq!(
            reader.received(buffer(b"boo\r\n\r\n")),
            Failed(UpgradeResponseReaderError::Incomplete)
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = String::from_utf8_lossy(UpgradeResponse::BYTES).replacen(
            "\r\n\r\n",
            &format!("\r\nReport-To: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(read_all(bytes.as_bytes()), Done(Buffer::new()));
    }
}
