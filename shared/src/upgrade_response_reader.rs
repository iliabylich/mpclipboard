use crate::{
    CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER, line_reader::LineReader,
    message::Message, prelude::*, strip_prefix_ignore_ascii_case,
};
use anyhow::{Context, Result, anyhow};
use core::num::NonZeroUsize;

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
        buf: [u8; Self::BUFFER_SIZE],
        len: NonZeroUsize,
    ) -> Completion<([u8; Self::BUFFER_SIZE], usize), anyhow::Error, ()> {
        let Some(buf) = buf.get(..len.get()) else {
            return Failed(anyhow!("given buffer is malformed"));
        };

        let mut leftover = [0; Self::BUFFER_SIZE];
        let mut leftover_len = 0;

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
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfResponse => {
                    self.seen_eos = true;

                    let Some(start) = pos.checked_add(1) else {
                        return Failed(anyhow!("buffer size is constant so it can't overflow"));
                    };
                    let Some(rest) = buf.get(start..) else {
                        return Failed(anyhow!("worst case is leftover is empty"));
                    };
                    let Some(dst) = leftover.get_mut(..rest.len()) else {
                        return Failed(anyhow!("leftover can't be longer than given buffer"));
                    };
                    dst.copy_from_slice(rest);
                    leftover_len = rest.len();
                    break;
                }
                HttpLine::Other => {}
            }
        }

        if self.try_finish() {
            Done((leftover, leftover_len))
        } else if self.seen_eos {
            Failed(anyhow!("got EOS but UpgradeResponse is incomplete"))
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
    fn parse(line: &[u8]) -> Result<Self> {
        let line = core::str::from_utf8(line).context("non-utf8 header")?;

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

#[cfg(test)]
mod tests {
    use super::UpgradeResponseReader;
    use crate::{
        test_helpers::{as_chunks_with_guaranteed_trailer, non_zero_usize},
        upgrade_response::UpgradeResponse,
    };
    use anyhow::{Context, Result};

    #[test]
    fn test_leftover() -> Result<()> {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeResponseReader::BUFFER_SIZE },
        >(UpgradeResponse::BYTES);

        let mut reader = UpgradeResponseReader::new();

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

        let levftover = reader
            .received(buf, len)
            .expect_done("trailer has been written");

        assert_eq!(
            levftover,
            (
                {
                    let mut buf = [0; _];
                    buf[0] = b'a';
                    buf[1] = b'b';
                    buf[2] = b'c';
                    buf
                },
                3,
            )
        );

        Ok(())
    }

    #[test]
    fn test_no_leftover() {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeResponseReader::BUFFER_SIZE },
        >(UpgradeResponse::BYTES);

        let mut reader = UpgradeResponseReader::new();

        for (buf, len) in chunks {
            reader
                .received(buf, len)
                .expect_pending("trailer hasn't been written yet");
        }

        let (buf, len) = trailer;
        let leftover = reader
            .received(buf, len)
            .expect_done("trailer has been written");

        assert_eq!(leftover, ([0; _], 0));
    }

    #[test]
    fn test_err() -> Result<()> {
        let mut reader = UpgradeResponseReader::new();

        let mut buf = [0; _];
        let malformed = b"boo\r\n\r\n";
        buf.get_mut(..malformed.len())
            .context("bug")?
            .copy_from_slice(malformed);
        let len = non_zero_usize(malformed.len())?;

        let err = reader
            .received(buf, len)
            .expect_failed("incomplete request");
        assert_eq!(err.to_string(), "got EOS but UpgradeResponse is incomplete");

        Ok(())
    }

    #[test]
    fn test_long_unknown_header() -> Result<()> {
        let head = core::str::from_utf8(UpgradeResponse::BYTES)?
            .strip_suffix("\r\n")
            .context("response must end with an empty line")?;
        let bytes = format!("{head}Report-To: {}\r\n\r\n", "a".repeat(300));

        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeResponseReader::BUFFER_SIZE },
        >(bytes.as_bytes());

        let mut reader = UpgradeResponseReader::new();
        for (buf, len) in chunks {
            reader
                .received(buf, len)
                .expect_pending("trailer hasn't been written yet");
        }

        let (buf, len) = trailer;
        let leftover = reader
            .received(buf, len)
            .expect_done("long unknown header is skipped");
        assert_eq!(leftover, ([0; _], 0));

        Ok(())
    }
}
