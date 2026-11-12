use crate::{
    config::Config, runtime::LoopContext, tls_stream::TlsWriteHalf, upgrade::UpgradeError,
};
use externally_driven_async_rt::io::{AsyncWrite, WriteAllError};
use mpclipboard_shared::{PROTOCOL_VERSION, UPGRADE_PROTOCOL};

pub(crate) struct UpgradeWriter<'s> {
    tx: TlsWriteHalf<'s>,
    request: String,
    written: usize,
}

impl<'s> UpgradeWriter<'s> {
    pub(crate) fn new(tx: TlsWriteHalf<'s>, config: &Config) -> Self {
        Self {
            tx,
            request: request(config),
            written: 0,
        }
    }

    pub(crate) async fn write(&mut self, ctx: &LoopContext<'_>) -> Result<(), UpgradeError> {
        while self.written < self.request.len() {
            match self
                .tx
                .write(ctx, &self.request.as_bytes()[self.written..])
                .await
            {
                Ok(0) => return Err(WriteAllError::WriteZero.into()),
                Ok(len) => self.written += len,
                Err(err) => return Err(WriteAllError::Io(err).into()),
            }
        }
        self.tx.flush(ctx).await?;
        Ok(())
    }
}

fn request(config: &Config) -> String {
    format!(
        "GET / HTTP/1.1\r\n\
Host: {host}:{port}\r\n\
Connection: Upgrade\r\n\
Upgrade: {UPGRADE_PROTOCOL}\r\n\
Token: {token}\r\n\
ID: {id}\r\n\
Version: {PROTOCOL_VERSION}\r\n\
\r\n",
        host = config.url.host(),
        port = config.url.port(),
        token = config.token.as_str(),
        id = config.id,
    )
}

#[cfg(test)]
mod tests {
    use super::request;
    use crate::config::Config;

    #[test]
    fn test_request() {
        let config = Config::new("https://localhost:8443", "sekret", "test-client").unwrap();
        assert_eq!(
            request(&config),
            "GET / HTTP/1.1\r\n\
Host: localhost:8443\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
Token: sekret\r\n\
ID: test-client\r\n\
Version: 6\r\n\
\r\n"
        );
    }
}
