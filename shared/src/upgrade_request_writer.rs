use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, HOST_PREFIX, ID_PREFIX, START_LINE, TOKEN_PREFIX,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest, VERSION_PREFIX, prelude::*,
};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter {
    buf: Buffer<{ UpgradeRequest::MAX_LENGTH }>,
}

impl UpgradeRequestWriter {
    pub fn new(req: UpgradeRequest) -> Self {
        let mut buf = Buffer::empty();

        let mut append = |s: &str| {
            for &byte in s.as_bytes() {
                if !buf.push(byte) {
                    unreachable!("UpgradeRequest is never longer than UpgradeRequest::MAX_LENGTH");
                }
            }
        };

        append(START_LINE);
        append("\r\n");

        append(HOST_PREFIX);
        append(req.host.as_str());
        append("\r\n");

        append(TOKEN_PREFIX);
        append(req.token.as_str());
        append("\r\n");

        append(ID_PREFIX);
        append(req.id.as_str());
        append("\r\n");

        append(VERSION_PREFIX);
        append(req.version.as_str());
        append("\r\n");

        append(CONNECTION_UPGRADE_HEADER);
        append("\r\n");

        append(UPGRADE_MPCLIPBOARD_RAW_HEADER);
        append("\r\n");

        append("\r\n");

        Self { buf }
    }

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn written(
        &mut self,
        n: NonZeroUsize,
    ) -> Result<Completion<(), ()>, UpgradeRequestWriterError> {
        self.buf
            .drop_n_front_bytes(n.get())
            .map_err(|remaining| UpgradeRequestWriterError {
                written: n.get(),
                remaining,
            })?;

        if self.remainder().is_empty() {
            Ok(Done(()))
        } else {
            Ok(Pending(()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("written() reported {written} bytes, but only {remaining} bytes remained")]
pub struct UpgradeRequestWriterError {
    pub written: usize,
    pub remaining: usize,
}

#[cfg(test)]
mod tests {
    use super::{UpgradeRequestWriter, UpgradeRequestWriterError};
    use crate::{
        HostPort, ID, Token, UpgradeRequest, Version, prelude::*, test_helpers::non_zero_usize,
    };

    const REQ: UpgradeRequest = UpgradeRequest {
        host: HostPort::const_new("localhost:3000"),
        token: Token::const_new("sekret"),
        id: ID::const_new("test-client"),
        version: Version::const_new("0.100.10"),
    };

    #[test]
    fn test_encode() {
        let writer = UpgradeRequestWriter::new(REQ);
        assert_eq!(
            core::str::from_utf8(writer.buf.as_slice()),
            Ok(
                "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nVersion: 0.100.10\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
            )
        );
    }

    #[test]
    fn test_write() {
        let mut writer = UpgradeRequestWriter::new(REQ);
        assert_eq!(
            core::str::from_utf8(writer.remainder()),
            Ok(
                "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nVersion: 0.100.10\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
            )
        );

        assert_eq!(writer.written(non_zero_usize(119)), Ok(Pending(())));
        assert_eq!(
            core::str::from_utf8(writer.remainder()),
            Ok("mpclipboard-raw\r\n\r\n")
        );

        assert_eq!(
            writer.written(non_zero_usize(writer.remainder().len())),
            Ok(Done(()))
        );
        assert_eq!(core::str::from_utf8(writer.remainder()), Ok(""));

        assert_eq!(
            writer.written(non_zero_usize(1)),
            Err(UpgradeRequestWriterError {
                written: 1,
                remaining: 0,
            })
        );
    }
}
