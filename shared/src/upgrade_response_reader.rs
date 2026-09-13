use crate::{
    CONNECTION_UPGRADE_HEADER, Completion, UPGRADE_MPCLIPBOARD_RAW_HEADER, message::Message,
    strip_prefix_ignore_ascii_case,
};
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
    ) -> Completion<([u8; UpgradeResponseReader::BUFFER_SIZE], usize), ()> {
        let Some(buf) = buf.get(..len.get()) else {
            return Completion::Failed;
        };

        for (pos, &byte) in buf.iter().enumerate() {
            if let Some(slot) = self.buf.get_mut(self.pos) {
                *slot = byte;
            } else {
                return Completion::Failed;
            }
            self.pos = self
                .pos
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("length overflow"));

            let Some(filled) = self.buf.get(..self.pos) else {
                return Completion::Failed;
            };

            let Some(line) = HttpLine::parse(filled) else {
                continue;
            };
            match line {
                HttpLine::StartLine => self.seen_start_line = true,
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfResponse => {
                    self.seen_eos = true;

                    let start = pos.checked_add(1).unwrap_or_else(|| {
                        unreachable!("buffer size is constant so it can't overflow")
                    });
                    let leftover = buf
                        .get(start..)
                        .unwrap_or_else(|| unreachable!("worst case is leftover is empty"));
                    self.buf = [0; _];
                    self.buf
                        .get_mut(..leftover.len())
                        .unwrap_or_else(|| {
                            unreachable!("leftover can't be longer than internal buffer")
                        })
                        .copy_from_slice(leftover);
                    self.pos = leftover.len();
                    break;
                }
                HttpLine::Other => {}
            }
            self.buf = [0; _];
            self.pos = 0;
        }

        if self.try_finish() {
            Completion::Done((self.buf, self.pos))
        } else if self.seen_eos {
            Completion::Failed
        } else {
            Completion::Pending(())
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
    fn parse(buf: &[u8]) -> Option<Self> {
        if !buf.ends_with(b"\r\n") {
            return None;
        }

        let end = buf
            .len()
            .checked_sub(2)
            .unwrap_or_else(|| unreachable!("len is >= 2"));
        let buf = buf
            .get(..end)
            .unwrap_or_else(|| unreachable!("buf contains at least two bytes"));
        let buf = core::str::from_utf8(buf).ok()?;

        if buf == "HTTP/1.1 101 Switching Protocols" {
            Some(Self::StartLine)
        } else if strip_prefix_ignore_ascii_case(buf, CONNECTION_UPGRADE_HEADER) == Some("") {
            Some(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(buf, UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("") {
            Some(Self::UpgradeMPClipboardRaw)
        } else if buf.is_empty() {
            Some(Self::EndOfResponse)
        } else {
            Some(Self::Other)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeResponseReader;
    use crate::{
        Completion, test_helpers::as_chunks_with_guaranteed_trailer,
        upgrade_response::UpgradeResponse,
    };
    use core::num::NonZeroUsize;

    #[test]
    fn test_leftover() {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeResponseReader::BUFFER_SIZE },
        >(UpgradeResponse::BYTES);

        let mut reader = UpgradeResponseReader::new();

        for (buf, len) in chunks {
            let res = reader.received(buf, len);
            assert_eq!(res, Completion::Pending(()));
        }

        let (mut buf, mut len) = trailer;
        buf[len.get()..len.get() + 3].copy_from_slice(b"abc");
        len = NonZeroUsize::new(len.get() + 3).unwrap();

        let res = reader.received(buf, len);

        assert_eq!(
            res,
            Completion::Done((
                {
                    let mut buf = [0; _];
                    buf[0] = b'a';
                    buf[1] = b'b';
                    buf[2] = b'c';
                    buf
                },
                3,
            ))
        );
    }

    #[test]
    fn test_no_leftover() {
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeResponseReader::BUFFER_SIZE },
        >(UpgradeResponse::BYTES);

        let mut reader = UpgradeResponseReader::new();

        for (buf, len) in chunks {
            let res = reader.received(buf, len);
            assert_eq!(res, Completion::Pending(()));
        }

        let (buf, len) = trailer;
        let res = reader.received(buf, len);

        assert_eq!(res, Completion::Done(([0; _], 0)));
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeResponseReader::new();

        let mut buf = [0; _];
        let malformed = b"boo\r\n\r\n";
        buf[..malformed.len()].copy_from_slice(malformed);
        let len = NonZeroUsize::new(malformed.len()).unwrap();

        assert_eq!(reader.received(buf, len), Completion::Failed);
    }
}
