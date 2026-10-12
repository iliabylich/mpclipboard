use crate::{
    Buffer, CONNECTION_UPGRADE_HEADER, CRLF, HOST_PREFIX, ID_PREFIX, MAX_HOST_PORT_LENGTH,
    MAX_ID_LENGTH, MAX_TOKEN_LENGTH, MAX_VERSION_LENGTH, START_LINE, TOKEN_PREFIX,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest, VERSION_PREFIX, prelude::*,
};
use core::num::NonZeroUsize;

macro_rules! layout_sizes {
    ($sizes:ident, $max:ident; $($kind:ident $x:ident => $m:expr),* $(,)?) => {
        const COMPONENTS_COUNT: usize = [$($m),*].len();

        #[expect(clippy::indexing_slicing)]
        const $sizes: [usize; COMPONENTS_COUNT + 1] = {
            let mut sizes = [0; COMPONENTS_COUNT + 1];
            let mut idx = 0;
            $(
                sizes[idx + 1] = sizes[idx] + $m;
                idx += 1;
            )*
            sizes
        };
        const $max: usize = {
            let [.., last] = $sizes;
            last
        };
    };
}

macro_rules! layout_step {
    ($sizes:ident, $req:ident, $buf:expr, $idx:expr;) => { $buf };
    ($sizes:ident, $req:ident, $buf:expr, $idx:expr; bytes $x:ident => $m:expr, $($rest:tt)*) => {
        layout_step!($sizes, $req, $buf.append_byte_array::<{ $m }, { $sizes[$idx] }>($x), $idx + 1; $($rest)*)
    };
    ($sizes:ident, $req:ident, $buf:expr, $idx:expr; string $field:ident => $m:expr, $($rest:tt)*) => {
        layout_step!($sizes, $req, $buf.append_non_empty_string::<{ $m }, { $sizes[$idx] }>(&$req.$field), $idx + 1; $($rest)*)
    };
}

macro_rules! layout_chain {
    ($sizes:ident, $req:ident; $($pieces:tt)*) => {
        layout_step!($sizes, $req, Buffer::<{ $sizes[0] }>::empty(), 1; $($pieces)*)
    };
}

macro_rules! layout {
    ($sizes:ident, $max:ident; $($pieces:tt)*) => {
        layout_sizes!($sizes, $max; $($pieces)*);

        fn encode(req: &UpgradeRequest) -> Buffer<$max> {
            layout_chain!($sizes, req; $($pieces)*)
        }
    };
}

layout! {
    SIZES, MAX_LENGTH;
    bytes START_LINE => START_LINE.len(),
    bytes CRLF => CRLF.len(),
    bytes HOST_PREFIX => HOST_PREFIX.len(),
    string host => MAX_HOST_PORT_LENGTH,
    bytes CRLF => CRLF.len(),
    bytes TOKEN_PREFIX => TOKEN_PREFIX.len(),
    string token => MAX_TOKEN_LENGTH,
    bytes CRLF => CRLF.len(),
    bytes ID_PREFIX => ID_PREFIX.len(),
    string id => MAX_ID_LENGTH,
    bytes CRLF => CRLF.len(),
    bytes VERSION_PREFIX => VERSION_PREFIX.len(),
    string version => MAX_VERSION_LENGTH,
    bytes CRLF => CRLF.len(),
    bytes CONNECTION_UPGRADE_HEADER => CONNECTION_UPGRADE_HEADER.len(),
    bytes CRLF => CRLF.len(),
    bytes UPGRADE_MPCLIPBOARD_RAW_HEADER => UPGRADE_MPCLIPBOARD_RAW_HEADER.len(),
    bytes CRLF => CRLF.len(),
    bytes CRLF => CRLF.len(),
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter {
    buf: Buffer<MAX_LENGTH>,
}

impl UpgradeRequestWriter {
    pub fn new(req: UpgradeRequest) -> Self {
        let buf = encode(&req);

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
