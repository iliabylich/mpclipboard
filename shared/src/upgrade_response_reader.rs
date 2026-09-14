use crate::{
    CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER, message::Message, prelude::*,
    strip_prefix_ignore_ascii_case,
};
use anyhow::{Context, Result, anyhow};
use core::num::NonZeroUsize;

#[expect(clippy::struct_excessive_bools)]
#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseReader {
    buf: [u8; Self::BUFFER_SIZE],
    pos: usize,

    seen_start_line: bool,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
    seen_eos: bool,
}

impl UpgradeResponseReader {
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;

    pub const fn new() -> Self {
        Self {
            buf: [0; _],
            pos: 0,

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

        for (pos, &byte) in buf.iter().enumerate() {
            let Some(slot) = self.buf.get_mut(self.pos) else {
                return Failed(anyhow!("internal buffer overflow"));
            };
            *slot = byte;

            let Some(nextpos) = self.pos.checked_add(1) else {
                return Failed(anyhow!("length overflow"));
            };
            self.pos = nextpos;

            let Some(filled) = self.buf.get(..self.pos) else {
                return Failed(anyhow!("internal buffer overflow"));
            };

            let line = match HttpLine::parse(filled) {
                Ok(Some(line)) => line,
                Ok(None) => continue,
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
                    let Some(leftover) = buf.get(start..) else {
                        return Failed(anyhow!("worst case is leftover is empty"));
                    };

                    self.buf = [0; _];
                    let Some(leftover_spot) = self.buf.get_mut(..leftover.len()) else {
                        return Failed(anyhow!("leftover can't be longer than internal buffer"));
                    };
                    leftover_spot.copy_from_slice(leftover);

                    self.pos = leftover.len();
                    break;
                }
                HttpLine::Other => {}
            }
            self.buf = [0; _];
            self.pos = 0;
        }

        if self.try_finish() {
            Done((self.buf, self.pos))
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
    fn parse(buf: &[u8]) -> Result<Option<Self>> {
        if !buf.ends_with(b"\r\n") {
            return Ok(None);
        }

        let end = buf.len().checked_sub(2).context("len is >= 2")?;
        let buf = buf.get(..end).context("buf contains at least two bytes")?;
        let buf = core::str::from_utf8(buf).context("non-utf8 header")?;

        if buf == "HTTP/1.1 101 Switching Protocols" {
            Ok(Some(Self::StartLine))
        } else if strip_prefix_ignore_ascii_case(buf, CONNECTION_UPGRADE_HEADER) == Some("") {
            Ok(Some(Self::ConnectionUpgrade))
        } else if strip_prefix_ignore_ascii_case(buf, UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("") {
            Ok(Some(Self::UpgradeMPClipboardRaw))
        } else if buf.is_empty() {
            Ok(Some(Self::EndOfResponse))
        } else {
            Ok(Some(Self::Other))
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
    use anyhow::Result;

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
        buf[len.get()..len.get() + 3].copy_from_slice(b"abc");
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
    fn test_no_leftover() -> Result<()> {
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

        Ok(())
    }

    #[test]
    fn test_err() -> Result<()> {
        let mut reader = UpgradeResponseReader::new();

        let mut buf = [0; _];
        let malformed = b"boo\r\n\r\n";
        buf[..malformed.len()].copy_from_slice(malformed);
        let len = non_zero_usize(malformed.len())?;

        let err = reader
            .received(buf, len)
            .expect_failed("incomplete request");
        assert_eq!(err.to_string(), "got EOS but UpgradeResponse is incomplete");

        Ok(())
    }
}
