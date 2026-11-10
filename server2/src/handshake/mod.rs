use crate::{
    ServerContext, fixed_str::FixedStr, message::Message, message_reader::MessageReader,
    write_all::write_all,
};
use core::time::Duration;
use externally_driven_async_rt::net::TcpStream;

mod parse;
use parse::parse;

pub(crate) type Token = FixedStr<20>;
pub(crate) type ID = FixedStr<20>;

const MAX_REQUEST_SIZE: usize = 2048;
const MAX_HEADERS: usize = 32;
const PROTOCOL_VERSION: &str = "6";
const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

const SWITCHING_PROTOCOLS: &[u8] =
    b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n";
const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

pub(crate) struct Handshake {
    buf: [u8; MAX_REQUEST_SIZE],
    len: usize,
}

impl Handshake {
    pub(crate) async fn read_write_with_timeout(
        ctx: &ServerContext<'_>,
        stream: &mut TcpStream,
        token: &Token,
        reader: &mut MessageReader,
        timeout: Duration,
    ) -> Result<(ID, Option<Message>), ()> {
        let handshake = Self::read_write(ctx, stream, token, reader);
        let Some(handshake) = ctx.timeout(timeout, handshake).await else {
            log::info!("no handshake within {}s", timeout.as_secs());
            return Err(());
        };
        handshake
    }

    async fn read_write(
        ctx: &ServerContext<'_>,
        stream: &mut TcpStream,
        token: &Token,
        reader: &mut MessageReader,
    ) -> Result<(ID, Option<Message>), ()> {
        let mut handshake = Self {
            buf: [0; MAX_REQUEST_SIZE],
            len: 0,
        };
        let (received_token, id, leftover) = handshake.read(ctx, stream).await?;

        let (_, mut tx) = stream.split();

        if received_token != *token {
            log::info!("[{id}] invalid token=\"{received_token}\"");
            let _ = write_all(ctx, &mut tx, UNAUTHORIZED).await;
            return Err(());
        }

        if write_all(ctx, &mut tx, SWITCHING_PROTOCOLS).await.is_err() {
            log::error!("[{id}] failed to send 101 Switching Protocols");
            return Err(());
        }

        let Ok(leftover_message) = reader.received(leftover) else {
            log::error!("[{id}] sent a malformed frame right after the request");
            return Err(());
        };
        Ok((id, leftover_message))
    }

    async fn read(
        &mut self,
        ctx: &ServerContext<'_>,
        stream: &mut TcpStream,
    ) -> Result<(Token, ID, &[u8]), ()> {
        loop {
            self.len += match stream.read(ctx, &mut self.buf[self.len..]).await {
                Ok(0) => {
                    log::info!("disconnected during handshake");
                    return Err(());
                }
                Ok(read) => read,
                Err(err) => {
                    log::error!("read() failed during handshake: {err}");
                    return Err(());
                }
            };

            match parse(&self.buf[..self.len]) {
                Ok(None) => {}
                Ok(Some((token, id, consumed))) => {
                    return Ok((token, id, &self.buf[consumed..self.len]));
                }
                Err(err) => {
                    log::error!("{err}");
                    return Err(());
                }
            }
        }
    }
}
