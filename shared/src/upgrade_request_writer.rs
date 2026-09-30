use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, HOST_PREFIX, ID_PREFIX, START_LINE, TOKEN_PREFIX,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest, VERSION_PREFIX, prelude::*,
};
use anyhow::{Context, Result, anyhow, ensure};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter {
    buf: Buffer<1_024>,
    pos: usize,
}

impl UpgradeRequestWriter {
    pub fn new(req: UpgradeRequest) -> Result<Self> {
        let mut buf = Buffer::new();

        let mut append = |s: &str| {
            for &byte in s.as_bytes() {
                ensure!(buf.push(byte), "must fit into 1kb");
            }
            Ok::<(), anyhow::Error>(())
        };

        append(START_LINE)?;
        append("\r\n")?;

        append(HOST_PREFIX)?;
        append(req.host.as_str())?;
        append("\r\n")?;

        append(TOKEN_PREFIX)?;
        append(req.token.as_str())?;
        append("\r\n")?;

        append(ID_PREFIX)?;
        append(req.id.as_str())?;
        append("\r\n")?;

        append(VERSION_PREFIX)?;
        append(req.version.as_str())?;
        append("\r\n")?;

        append(CONNECTION_UPGRADE_HEADER)?;
        append("\r\n")?;

        append(UPGRADE_MPCLIPBOARD_RAW_HEADER)?;
        append("\r\n")?;

        append("\r\n")?;

        Ok(Self { buf, pos: 0 })
    }

    pub fn remainder(&self) -> Result<&[u8]> {
        self.buf
            .as_slice()
            .get(self.pos..)
            .context("malformed internal state")
    }

    pub fn written(&mut self, n: NonZeroUsize) -> Completion<(), anyhow::Error, ()> {
        let Some(nextpos) = self.pos.checked_add(n.get()) else {
            return Failed(anyhow!("pos overflow"));
        };
        self.pos = nextpos;

        match self.pos.cmp(&self.buf.as_slice().len()) {
            core::cmp::Ordering::Less => Pending(()),
            core::cmp::Ordering::Equal => Done(()),
            core::cmp::Ordering::Greater => Failed(anyhow!("buffer overflow")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeRequestWriter;
    use crate::{HostPort, ID, Token, UpgradeRequest, Version, test_helpers::non_zero_usize};
    use anyhow::Result;

    fn req() -> Result<UpgradeRequest> {
        Ok(UpgradeRequest {
            host: HostPort::new("localhost:3000")?,
            token: Token::new("sekret")?,
            id: ID::new("test-client")?,
            version: Version::new("0.100.10")?,
        })
    }

    #[test]
    fn test_encode() -> Result<()> {
        let writer = UpgradeRequestWriter::new(req()?)?;
        assert_eq!(
            core::str::from_utf8(writer.buf.as_slice())?,
            "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nVersion: 0.100.10\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );

        Ok(())
    }

    #[test]
    fn test_write() -> Result<()> {
        let mut writer = UpgradeRequestWriter::new(req()?)?;
        assert_eq!(
            core::str::from_utf8(writer.remainder()?)?,
            "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nVersion: 0.100.10\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );

        writer
            .written(non_zero_usize(119)?)
            .expect_pending("we've written only 100 bytes");
        assert_eq!(
            core::str::from_utf8(writer.remainder()?)?,
            "mpclipboard-raw\r\n\r\n"
        );

        writer
            .written(non_zero_usize(writer.remainder()?.len())?)
            .expect_done("we've written a full request");
        assert_eq!(writer.remainder()?, b"");

        let err = writer
            .written(non_zero_usize(1)?)
            .expect_failed("trying to go past buffer end");
        assert_eq!(err.to_string(), "buffer overflow");

        Ok(())
    }
}
