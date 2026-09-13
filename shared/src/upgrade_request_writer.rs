use crate::{
    CONNECTION_UPGRADE_HEADER, Completion, HOST_PREFIX, ID_PREFIX, START_LINE, TOKEN_PREFIX,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, UpgradeRequest,
};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeRequestWriter {
    buf: [u8; 1_024],
    len: usize,
    pos: usize,
}

impl UpgradeRequestWriter {
    pub fn new(req: UpgradeRequest) -> Self {
        let mut buf = [0; 1_024];
        let mut pos = 0;

        let mut append = |pos: &mut usize, s: &str| {
            let start = *pos;
            let end = start
                .checked_add(s.len())
                .unwrap_or_else(|| unreachable!("length overflow"));
            buf.get_mut(start..end)
                .unwrap_or_else(|| unreachable!("must fit into 1kb"))
                .copy_from_slice(s.as_bytes());
            *pos = end;
        };

        append(&mut pos, START_LINE);
        append(&mut pos, "\r\n");

        append(&mut pos, HOST_PREFIX);
        append(&mut pos, req.host.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, TOKEN_PREFIX);
        append(&mut pos, req.token.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, ID_PREFIX);
        append(&mut pos, req.id.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, CONNECTION_UPGRADE_HEADER);
        append(&mut pos, "\r\n");

        append(&mut pos, UPGRADE_MPCLIPBOARD_RAW_HEADER);
        append(&mut pos, "\r\n");

        append(&mut pos, "\r\n");

        Self {
            buf,
            len: pos,
            pos: 0,
        }
    }

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.buf
            .get(self.pos..self.len)
            .unwrap_or_else(|| unreachable!("malformed internal state"))
    }

    pub fn written(&mut self, n: NonZeroUsize) -> Completion<(), ()> {
        self.pos = self
            .pos
            .checked_add(n.get())
            .unwrap_or_else(|| unreachable!("pos overflow"));

        match self.pos.cmp(&self.len) {
            core::cmp::Ordering::Less => Completion::Pending(()),
            core::cmp::Ordering::Equal => Completion::Done(()),
            core::cmp::Ordering::Greater => Completion::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeRequestWriter;
    use crate::{Completion, HostPort, ID, Token, UpgradeRequest};
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
        let writer = UpgradeRequestWriter::new(req());
        assert_eq!(
            &writer.buf[..writer.len],
            b"GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );
    }

    #[test]
    fn test_write() {
        let mut writer = UpgradeRequestWriter::new(req());
        assert_eq!(
            core::str::from_utf8(writer.remainder()).unwrap(),
            "GET / HTTP/1.1\r\nHost: localhost:3000\r\nToken: sekret\r\nID: test-client\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n"
        );

        assert_eq!(
            writer.written(NonZeroUsize::new(100).unwrap()),
            Completion::Pending(())
        );
        assert_eq!(
            core::str::from_utf8(writer.remainder()).unwrap(),
            "mpclipboard-raw\r\n\r\n"
        );

        assert_eq!(
            writer.written(NonZeroUsize::new(writer.remainder().len()).unwrap()),
            Completion::Done(())
        );
        assert_eq!(writer.remainder(), b"");

        assert_eq!(
            writer.written(NonZeroUsize::new(1).unwrap()),
            Completion::Failed
        );
    }
}
