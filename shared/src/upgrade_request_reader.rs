use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX, Message, START_LINE,
    TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest, prelude::*,
    strip_prefix_ignore_ascii_case,
};
use anyhow::anyhow;
use core::num::NonZeroUsize;

#[expect(clippy::struct_excessive_bools)]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpgradeRequestReader {
    buf: [u8; Self::BUFFER_SIZE],
    pos: usize,

    seen_start_line: bool,
    host: Option<HostPort>,
    token: Option<Token>,
    id: Option<ID>,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
    seen_eos: bool,
}

impl UpgradeRequestReader {
    pub const BUFFER_SIZE: usize = Message::BYTESIZE - 1;

    pub const fn new() -> Self {
        Self {
            buf: [0; _],
            pos: 0,

            seen_start_line: false,
            host: None,
            token: None,
            id: None,
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
            if let Some(slot) = self.buf.get_mut(self.pos) {
                *slot = byte;
            } else {
                return Failed(anyhow!("buffer overflow"));
            }
            self.pos = self
                .pos
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("length overflow"));

            let Some(filled) = self.buf.get(..self.pos) else {
                return Failed(anyhow!("buffer overflow"));
            };

            let Some(line) = HttpLine::parse(filled) else {
                continue;
            };
            match line {
                HttpLine::StartLine => self.seen_start_line = true,
                HttpLine::HostPort(host) => self.host = Some(host),
                HttpLine::Token(token) => self.token = Some(token),
                HttpLine::ID(id) => self.id = Some(id),
                HttpLine::ConnectionUpgrade => self.seen_connection_upgrade = true,
                HttpLine::UpgradeMPClipboardRaw => self.seen_upgrade_mpclipboard_raw = true,
                HttpLine::EndOfRequest => {
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

        if let Some(req) = self.try_finish() {
            if self.pos != 0 {
                return Failed(anyhow!("got leftover in UpgradeRequestReader"));
            }
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
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
            && self.seen_eos
        {
            Some(UpgradeRequest { host, token, id })
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
    ConnectionUpgrade,
    UpgradeMPClipboardRaw,
    EndOfRequest,

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
        let line = core::str::from_utf8(buf).ok()?;

        if line == START_LINE {
            Some(Self::StartLine)
        } else if let Some(host) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX) {
            let host = HostPort::new(host).ok()?;
            Some(Self::HostPort(host))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX) {
            let token = Token::new(value).ok()?;
            Some(Self::Token(token))
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX) {
            let id = ID::new(value).ok()?;
            Some(Self::ID(id))
        } else if strip_prefix_ignore_ascii_case(line, CONNECTION_UPGRADE_HEADER) == Some("") {
            Some(Self::ConnectionUpgrade)
        } else if strip_prefix_ignore_ascii_case(line, UPGRADE_MPCLIPBOARD_RAW_HEADER) == Some("") {
            Some(Self::UpgradeMPClipboardRaw)
        } else if line.is_empty() {
            Some(Self::EndOfRequest)
        } else {
            Some(Self::Other)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeRequestReader;
    use crate::{
        HostPort, ID, Token, UpgradeRequest, UpgradeRequestWriter,
        test_helpers::as_chunks_with_guaranteed_trailer,
    };
    use core::num::NonZeroUsize;

    fn new_reqwest() -> UpgradeRequest {
        UpgradeRequest {
            host: HostPort::new("localhost:3000").unwrap(),
            token: Token::new("sekret").unwrap(),
            id: ID::new("test-client").unwrap(),
        }
    }

    #[test]
    fn test_leftover() {
        let w = UpgradeRequestWriter::new(new_reqwest());
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeRequestReader::BUFFER_SIZE },
        >(w.remainder());

        let mut reader = UpgradeRequestReader::new();

        for (buf, len) in chunks {
            reader.received(buf, len).unwrap_pending();
        }

        let (mut buf, mut len) = trailer;
        buf[len.get()..len.get() + 3].copy_from_slice(b"abc");
        len = NonZeroUsize::new(len.get() + 3).unwrap();

        let err = reader.received(buf, len).unwrap_err();
        assert_eq!(err.to_string(), "got leftover in UpgradeRequestReader");
    }

    #[test]
    fn test_no_leftover() {
        let w = UpgradeRequestWriter::new(new_reqwest());
        let (chunks, trailer) = as_chunks_with_guaranteed_trailer::<
            { UpgradeRequestReader::BUFFER_SIZE },
        >(w.remainder());

        let mut reader = UpgradeRequestReader::new();

        for (buf, len) in chunks {
            reader.received(buf, len).unwrap_pending();
        }

        let (buf, len) = trailer;
        let req = reader.received(buf, len).unwrap();

        assert_eq!(req, new_reqwest());
    }

    #[test]
    fn test_err() {
        let mut reader = UpgradeRequestReader::new();

        let mut buf = [0; _];
        let malformed = b"boo\r\n\r\n";
        buf[..malformed.len()].copy_from_slice(malformed);
        let len = NonZeroUsize::new(malformed.len()).unwrap();

        let err = reader.received(buf, len).unwrap_err();

        assert_eq!(err.to_string(), "got EOS but no complete UpgradeRequest");
    }
}
