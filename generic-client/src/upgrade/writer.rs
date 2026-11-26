use crate::{
    config::Config,
    tls_stream::{TlsError, TlsWriteHalf},
};
use core::fmt::Write;
use externally_driven_async_rt::io::AsyncWrite;
use mpclipboard_shared::{FormatBuffer, PROTOCOL_VERSION, UPGRADE_PROTOCOL};

pub(crate) struct UpgradeWriter<'s> {
    tx: TlsWriteHalf<'s>,
    buf: FormatBuffer<1024>,
    written: usize,
}

impl<'s> UpgradeWriter<'s> {
    pub(crate) fn new(tx: TlsWriteHalf<'s>, config: &Config) -> Self {
        Self {
            tx,
            buf: format_request(config),
            written: 0,
        }
    }

    pub(crate) async fn write(&mut self) -> Result<(), UpgradeWriterError> {
        while self.written < self.buf.as_bytes().len() {
            match self.tx.write(&self.buf.as_bytes()[self.written..]).await {
                Ok(0) => return Err(UpgradeWriterError::WriteZero),
                Ok(len) => self.written += len,
                Err(err) => return Err(UpgradeWriterError::Io(err)),
            }
        }
        self.tx.flush().await?;
        Ok(())
    }
}

fn format_request(config: &Config) -> FormatBuffer<1024> {
    let mut request = FormatBuffer::new();
    write!(
        request,
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
    .unwrap_or_else(|_| unreachable!("upgrade request exceeds 1024 bytes"));
    request
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UpgradeWriterError {
    #[error(transparent)]
    Io(#[from] TlsError),
    #[error("TLS stream accepted zero bytes")]
    WriteZero,
}

#[cfg(test)]
mod tests {
    use super::format_request;
    use crate::config::Config;

    #[test]
    fn test_request() {
        let config = Config::new("https://localhost:8443", "sekret", "test-client").unwrap();
        assert_eq!(
            format_request(&config).as_bytes(),
            b"GET / HTTP/1.1\r\n\
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
