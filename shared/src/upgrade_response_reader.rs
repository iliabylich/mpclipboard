use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    line_reader::{LineReader, LineReaderError},
    message::MessageSize,
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
#[derive(Debug, Clone)]
pub struct UpgradeResponseReader {
    lines: LineReader<BufferSize>,

    seen_start_line: bool,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
    seen_eos: bool,
}

impl UpgradeResponseReader {
    pub fn new() -> Self {
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
        mut buf: Buffer<BufferSize>,
    ) -> Result<Completion<Buffer<BufferSize>, ()>, UpgradeResponseReaderError> {
        buf.drain_front::<UpgradeResponseReaderError>(|byte| {
            let Done(line) = self.lines.push(byte)? else {
                return Ok(Continue(()));
            };

            let line = HttpLine::parse(line.as_slice())?;

            match line {
                HttpLine::StartLine => self.seen_start_line = true,
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfResponse => {
                    self.seen_eos = true;
                    return Ok(Break(()));
                }
                HttpLine::Other => {}
            }

            Ok(Continue(()))
        })?;

        if self.try_finish() {
            Ok(Done(buf))
        } else if self.seen_eos {
            Err(UpgradeResponseReaderError::Incomplete)
        } else {
            Ok(Pending(()))
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
        let line = core::str::from_utf8(line)?;

        if line == "HTTP/1.1 101 Switching Protocols" {
            Ok(Self::StartLine)
        } else if strip_prefix_ignore_ascii_case(line, &CONNECTION_UPGRADE_HEADER) == Some("") {
            Ok(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(line, &UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("")
        {
            Ok(Self::UpgradeMPClipboardRaw)
        } else if line.is_empty() {
            Ok(Self::EndOfResponse)
        } else {
            Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UpgradeResponseReaderError {
    #[error("{0}")]
    Line(#[from] LineReaderError),
    #[error("non-utf8 header: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("got EOS but UpgradeResponse is incomplete")]
    Incomplete,
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use super::{UpgradeResponseReader, UpgradeResponseReaderError};
    use crate::{
        Buffer,
        prelude::*,
        test_helpers::{as_chunks_with_guaranteed_trailer, buffer},
        upgrade_response::UpgradeResponse,
    };

    fn read_all(
        bytes: &[u8],
    ) -> Result<Completion<Buffer<super::BufferSize>, ()>, UpgradeResponseReaderError> {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<super::BufferSize>(bytes);

        let mut reader = UpgradeResponseReader::new();
        for buf in chunks {
            assert_eq!(reader.received(buf), Ok(Pending(())));
        }
        reader.received(trailer)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(UpgradeResponse::BYTES), Ok(Done(Buffer::empty())));
    }

    #[test]
    fn test_leftover() {
        let bytes = [UpgradeResponse::BYTES, b"abc"].concat();
        assert_eq!(read_all(&bytes), Ok(Done(buffer(b"abc"))));
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeResponseReader::new();
        assert_eq!(
            reader.received(buffer(b"boo\r\n\r\n")),
            Err(UpgradeResponseReaderError::Incomplete)
        );
    }

    #[test]
    fn test_long_unknown_header() {
        let bytes = String::from_utf8_lossy(UpgradeResponse::BYTES).replacen(
            "\r\n\r\n",
            &format!("\r\nReport-To: {}\r\n\r\n", "a".repeat(300)),
            1,
        );
        assert_eq!(read_all(bytes.as_bytes()), Ok(Done(Buffer::empty())));
    }
}
