use crate::{
    ServerContext, constants::UPGRADE_REQUEST_BUFFER_SIZE, errors::ClientError,
    upgrade::parse::parse,
};
use externally_driven_async_rt::{io::AsyncRead, net::BorrowedReadHalf};
use mpclipboard_shared::{ID, Token};

pub(crate) struct UpgradeReader<'s> {
    rx: BorrowedReadHalf<'s>,
    buf: [u8; UPGRADE_REQUEST_BUFFER_SIZE],
    len: usize,
}

impl<'s> UpgradeReader<'s> {
    pub(crate) const fn new(rx: BorrowedReadHalf<'s>) -> Self {
        Self {
            rx,
            buf: [0; UPGRADE_REQUEST_BUFFER_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn read(
        &mut self,
        ctx: &ServerContext<'_>,
    ) -> Result<(Token, ID), ClientError> {
        loop {
            match self.rx.read(ctx, &mut self.buf[self.len..]).await {
                Ok(0) => return Err(ClientError::DisconnectedDuringUpgrade),
                Ok(read) => self.len += read,
                Err(err) => return Err(ClientError::UpgradeRead(err)),
            }

            if let Some((token, id, consumed)) = parse(&self.buf[..self.len])? {
                if consumed != self.len {
                    return Err(ClientError::BytesAfterUpgrade { id });
                }
                return Ok((token, id));
            }
        }
    }
}
