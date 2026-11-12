use crate::{
    constants::UPGRADE_RESPONSE_BUFFER_SIZE,
    runtime::LoopContext,
    tls_stream::TlsReadHalf,
    upgrade::{UpgradeError, parse::parse},
};
use externally_driven_async_rt::io::AsyncReadExt;

pub(crate) struct UpgradeReader<'s> {
    rx: TlsReadHalf<'s>,
    buf: [u8; UPGRADE_RESPONSE_BUFFER_SIZE],
    len: usize,
}

impl<'s> UpgradeReader<'s> {
    pub(crate) const fn new(rx: TlsReadHalf<'s>) -> Self {
        Self {
            rx,
            buf: [0; UPGRADE_RESPONSE_BUFFER_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn read(&mut self, ctx: &LoopContext<'_>) -> Result<(), UpgradeError> {
        loop {
            self.rx
                .read_exact(ctx, &mut self.buf[self.len..=self.len])
                .await?;
            self.len += 1;

            if parse(&self.buf[..self.len])? {
                return Ok(());
            }
        }
    }
}
