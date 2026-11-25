use crate::{
    BorrowedReadHalf, constants::UPGRADE_REQUEST_BUFFER_SIZE, errors::ClientError,
    upgrade::parse::parse,
};
use externally_driven_async_rt::io::AsyncRead;
use mpclipboard_shared::{ID, Token};

pub(crate) struct UpgradeReader<'s, 'sel> {
    rx: BorrowedReadHalf<'s, 'sel>,
    buf: [u8; UPGRADE_REQUEST_BUFFER_SIZE],
    len: usize,
}

impl<'s, 'sel> UpgradeReader<'s, 'sel> {
    pub(crate) const fn new(rx: BorrowedReadHalf<'s, 'sel>) -> Self {
        Self {
            rx,
            buf: [0; UPGRADE_REQUEST_BUFFER_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn read(&mut self) -> Result<(Token, ID), ClientError> {
        loop {
            match self.rx.read(&mut self.buf[self.len..]).await {
                Ok(0) => return Err(ClientError::DisconnectedDuringUpgrade),
                Ok(read) => self.len += read,
                Err(err) => return Err(ClientError::UpgradeRead(err)),
            }

            if let Some((token, id, consumed)) =
                parse(&self.buf[..self.len]).map_err(ClientError::Upgrade)?
            {
                if consumed != self.len {
                    return Err(ClientError::BytesAfterUpgrade(id));
                }
                return Ok((token, id));
            }
        }
    }
}
