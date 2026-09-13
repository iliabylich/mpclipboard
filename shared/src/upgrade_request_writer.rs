use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, ID_PREFIX, START_LINE, TOKEN_PREFIX,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest,
};
use core::num::NonZeroUsize;

#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter<const N: usize> {
    buf: [u8; N],
    len: usize,
    pos: usize,
}

impl<const N: usize> UpgradeRequestWriter<N> {
    pub fn new(req: UpgradeRequest, mut buf: [u8; N]) -> Result<Self, UpgradeRequestWriterError> {
        let mut pos = 0;

        let mut append = |pos: &mut usize, s: &str| -> Result<(), UpgradeRequestWriterError> {
            let start = *pos;
            let end = start
                .checked_add(s.len())
                .unwrap_or_else(|| unreachable!("length overflow"));
            buf.get_mut(start..end)
                .ok_or(UpgradeRequestWriterError::BufferIsTooSmall)?
                .copy_from_slice(s.as_bytes());
            *pos = end;
            Ok(())
        };

        append(&mut pos, START_LINE)?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, HOST_PREFIX)?;
        append(&mut pos, req.host.as_str())?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, TOKEN_PREFIX)?;
        append(&mut pos, req.token.as_str())?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, ID_PREFIX)?;
        append(&mut pos, req.id.as_str())?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, CONNECTION_UPGRADE_HEADER)?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, UPGRADE_MPCLIPBOARD_RAW_HEADER)?;
        append(&mut pos, "\r\n")?;

        append(&mut pos, "\r\n")?;

        Ok(Self {
            buf,
            len: pos,
            pos: 0,
        })
    }

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.buf
            .get(self.pos..self.len)
            .unwrap_or_else(|| unreachable!("malformed internal state"))
    }

    pub fn written(&mut self, n: NonZeroUsize) -> UpgradeRequestWriterResult {
        self.pos = self
            .pos
            .checked_add(n.get())
            .unwrap_or_else(|| unreachable!("pos overflow"));

        match self.pos.cmp(&self.len) {
            core::cmp::Ordering::Less => UpgradeRequestWriterResult::Pending,
            core::cmp::Ordering::Equal => UpgradeRequestWriterResult::Done,
            core::cmp::Ordering::Greater => UpgradeRequestWriterResult::Error,
        }
    }
}

#[must_use]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UpgradeRequestWriterResult {
    Done,
    Pending,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpgradeRequestWriterError {
    BufferIsTooSmall,
}

impl core::fmt::Display for UpgradeRequestWriterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferIsTooSmall => f.write_str("buffer is too small"),
        }
    }
}

impl core::error::Error for UpgradeRequestWriterError {}

#[cfg(test)]
mod tests {
    use super::{UpgradeRequestWriter, UpgradeRequestWriterError, UpgradeRequestWriterResult};
    use crate::{HostPort, ID, Token, UpgradeRequest};
    use core::num::NonZeroUsize;

    fn req() -> UpgradeRequest {
        UpgradeRequest {
            host: HostPort::new("localhost:3000").unwrap(),
            token: Token::new("sekret").unwrap(),
            id: ID::new("test-client").unwrap(),
        }
    }

    #[test]
    fn test_encode() {
        let writer = UpgradeRequestWriter::new(req(), [0; 200]).unwrap();
        assert_eq!(
            &writer.buf[..writer.len],
            b"GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );

        assert_eq!(
            UpgradeRequestWriter::new(req(), [0; 5]).unwrap_err(),
            UpgradeRequestWriterError::BufferIsTooSmall
        );
    }

    #[test]
    fn test_write() {
        let mut writer = UpgradeRequestWriter::new(req(), [0; 200]).unwrap();
        assert_eq!(
            core::str::from_utf8(writer.remainder()).unwrap(),
            "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );

        assert_eq!(
            writer.written(NonZeroUsize::new(100).unwrap()),
            UpgradeRequestWriterResult::Pending
        );
        assert_eq!(
            core::str::from_utf8(writer.remainder()).unwrap(),
            "mpclipboard-raw\r\n\r\n"
        );

        assert_eq!(
            writer.written(NonZeroUsize::new(writer.remainder().len()).unwrap()),
            UpgradeRequestWriterResult::Done
        );
        assert_eq!(writer.remainder(), b"");

        assert_eq!(
            writer.written(NonZeroUsize::new(1).unwrap()),
            UpgradeRequestWriterResult::Error
        );
    }
}
