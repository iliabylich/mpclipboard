use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, CRLF, ConnectionUpgradeHeaderLength, CrlfLength,
    HOST_PREFIX, HostPrefixLength, ID_PREFIX, IdPrefixLength, MaxHostPortLength, MaxIdLength,
    MaxTokenLength, MaxVersionLength, START_LINE, StartLineLength, TOKEN_PREFIX, TokenPrefixLength,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeMpclipboardRawHeaderLength, UpgradeRequest,
    VERSION_PREFIX, VersionPrefixLength, prelude::*,
};
use core::num::NonZeroUsize;
use typenum::{U0, op};

type StartLineWithCrlfLength = op!(StartLineLength + CrlfLength);
type HostLineLength = op!(HostPrefixLength + MaxHostPortLength + CrlfLength);
type TokenLineLength = op!(TokenPrefixLength + MaxTokenLength + CrlfLength);
type IdLineLength = op!(IdPrefixLength + MaxIdLength + CrlfLength);
type VersionLineLength = op!(VersionPrefixLength + MaxVersionLength + CrlfLength);
type ConnectionLineLength = op!(ConnectionUpgradeHeaderLength + CrlfLength);
type UpgradeLineLength = op!(UpgradeMpclipboardRawHeaderLength + CrlfLength);

type MaxLength = op!(StartLineWithCrlfLength
    + HostLineLength
    + TokenLineLength
    + IdLineLength
    + VersionLineLength
    + ConnectionLineLength
    + UpgradeLineLength
    + CrlfLength);

#[must_use]
#[derive(Debug, Clone)]
pub struct UpgradeRequestWriter {
    buf: Buffer<MaxLength>,
}

impl UpgradeRequestWriter {
    pub fn new(req: &UpgradeRequest) -> Self {
        let buf: Buffer<MaxLength> = Buffer::<U0>::empty()
            .append_byte_array(&START_LINE)
            .append_byte_array(&CRLF)
            .append_byte_array(&HOST_PREFIX)
            .append_non_empty_string(&req.host)
            .append_byte_array(&CRLF)
            .append_byte_array(&TOKEN_PREFIX)
            .append_non_empty_string(&req.token)
            .append_byte_array(&CRLF)
            .append_byte_array(&ID_PREFIX)
            .append_non_empty_string(&req.id)
            .append_byte_array(&CRLF)
            .append_byte_array(&VERSION_PREFIX)
            .append_non_empty_string(&req.version)
            .append_byte_array(&CRLF)
            .append_byte_array(&CONNECTION_UPGRADE_HEADER)
            .append_byte_array(&CRLF)
            .append_byte_array(&UPGRADE_MPCLIPBOARD_RAW_HEADER)
            .append_byte_array(&CRLF)
            .append_byte_array(&CRLF);

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
        let writer = UpgradeRequestWriter::new(&REQ);
        assert_eq!(
            core::str::from_utf8(writer.buf.as_slice()),
            Ok(
                "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nVersion: 0.100.10\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
            )
        );
    }

    #[test]
    fn test_write() {
        let mut writer = UpgradeRequestWriter::new(&REQ);
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
