use crate::{
    config::Config,
    tls_stream::{TlsError, TlsWriteHalf},
};
use core::fmt::Write;
use externally_driven_async_rt::io::AsyncWrite;
use mpclipboard_shared::{PROTOCOL_VERSION, UPGRADE_PROTOCOL};

pub(crate) struct UpgradeWriter<'s> {
    tx: TlsWriteHalf<'s>,
    buf: Buffer,
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
        while self.written < self.buf.len() {
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

fn format_request(config: &Config) -> Buffer {
    let mut request = Buffer::new();
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

struct Buffer {
    buf: [u8; 1024],
    len: usize,
}

impl Buffer {
    const fn new() -> Self {
        Self {
            buf: [0; 1024],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    const fn len(&self) -> usize {
        self.len
    }
}

impl Write for Buffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let Some(spare) = self.buf.get_mut(self.len..self.len + s.len()) else {
            return Err(core::fmt::Error);
        };
        spare.copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
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
