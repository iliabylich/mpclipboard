use crate::ServerContext;
use externally_driven_async_rt::{
    io::{AsyncWrite, WriteAllError},
    net::BorrowedWriteHalf,
};
use rustix::io::Errno;

const SWITCHING_PROTOCOLS_RESPONSE: &str = concat!(
    "HTTP/1.1 101 Switching Protocols\r\n",
    "Connection: Upgrade\r\n",
    "Upgrade: mpclipboard-raw\r\n",
    "\r\n",
);

const UNAUTHORIZED_RESPONSE: &str = concat!(
    "HTTP/1.1 401 Unauthorized\r\n",
    "Content-Length: 0\r\n",
    "\r\n",
);

pub(crate) struct UpgradeWriter<'s> {
    tx: BorrowedWriteHalf<'s>,
    response: &'static [u8],
    written: usize,
}

impl<'s> UpgradeWriter<'s> {
    pub(crate) const fn switching_protocols(tx: BorrowedWriteHalf<'s>) -> Self {
        Self::new(tx, SWITCHING_PROTOCOLS_RESPONSE)
    }

    pub(crate) const fn unauthorized(tx: BorrowedWriteHalf<'s>) -> Self {
        Self::new(tx, UNAUTHORIZED_RESPONSE)
    }

    const fn new(tx: BorrowedWriteHalf<'s>, response: &'static str) -> Self {
        Self {
            tx,
            response: response.as_bytes(),
            written: 0,
        }
    }

    pub(crate) async fn write(
        &mut self,
        ctx: &ServerContext<'_>,
    ) -> Result<(), WriteAllError<Errno>> {
        while self.written < self.response.len() {
            match self.tx.write(ctx, &self.response[self.written..]).await {
                Ok(0) => return Err(WriteAllError::WriteZero),
                Ok(len) => self.written += len,
                Err(err) => return Err(WriteAllError::Io(err)),
            }
        }
        Ok(())
    }
}
