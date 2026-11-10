use crate::{
    ServerContext, client::Client, constants::HANDSHAKE_REQUEST_SIZE, id::ID, message::Message,
    message_reader::MessageReader, pubsub::PubSubHandle, token::Token, write_all::write_all,
};
use core::time::Duration;
use externally_driven_async_rt::net::TcpStream;

pub(crate) struct Stranger<'a> {
    stream: TcpStream,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
    buf: [u8; HANDSHAKE_REQUEST_SIZE],
    len: usize,
}

impl<'a> Stranger<'a> {
    pub(crate) fn new(stream: TcpStream, token: &'a Token, pubsub: PubSubHandle<'a>) -> Self {
        Self {
            stream,
            token,
            pubsub,
            buf: [0; HANDSHAKE_REQUEST_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn handshake(
        mut self,
        ctx: &ServerContext<'_>,
        timeout: Duration,
    ) -> Result<Client<'a>, ()> {
        let mut reader = MessageReader::new();

        let read_write = self.read_write(ctx, &mut reader);
        let Some(read_write) = ctx.timeout(timeout, read_write).await else {
            log::info!("no handshake within {}s", timeout.as_secs());
            return Err(());
        };
        let (id, leftover_message) = read_write?;
        log::info!("[{id}] connected");

        if let Some(message) = leftover_message {
            self.pubsub.publish(id, message);
        }

        Ok(Client::new(self.stream, reader, id, self.pubsub))
    }

    async fn read_write(
        &mut self,
        ctx: &ServerContext<'_>,
        reader: &mut MessageReader,
    ) -> Result<(ID, Option<Message>), ()> {
        let (received_token, id, consumed) = self.read(ctx).await?;

        if let Err(wrong) = self.token.compare(&received_token) {
            self.write_401(ctx, id, wrong).await;
            return Err(());
        }

        self.write_101(ctx, id).await?;

        let Ok(leftover_message) = reader.received(&self.buf[consumed..self.len]) else {
            log::error!("[{id}] sent a malformed frame right after the request");
            return Err(());
        };
        Ok((id, leftover_message))
    }

    async fn read(&mut self, ctx: &ServerContext<'_>) -> Result<(Token, ID, usize), ()> {
        loop {
            match self.stream.read(ctx, &mut self.buf[self.len..]).await {
                Ok(0) => {
                    log::info!("disconnected during handshake");
                    return Err(());
                }
                Ok(read) => {
                    self.len += read;
                }
                Err(err) => {
                    log::error!("read() failed during handshake: {err}");
                    return Err(());
                }
            };

            if let Some(parsed) = crate::handshake::parse(&self.buf[..self.len])? {
                return Ok(parsed);
            }
        }
    }

    async fn write_101(&mut self, ctx: &ServerContext<'_>, id: ID) -> Result<(), ()> {
        const SWITCHING_PROTOCOLS_RESPONSE: &str = concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "\r\n",
        );

        let (_, mut tx) = self.stream.split();

        if write_all(ctx, &mut tx, SWITCHING_PROTOCOLS_RESPONSE.as_bytes())
            .await
            .is_err()
        {
            log::error!("[{id}] failed to send 101 Switching Protocols");
            return Err(());
        }

        Ok(())
    }

    async fn write_401(&mut self, ctx: &ServerContext<'_>, id: ID, wrong: &str) {
        const UNAUTHORIZED_RESPONSE: &str = concat!(
            "HTTP/1.1 401 Unauthorized\r\n",
            "Content-Length: 0\r\n",
            "\r\n",
        );

        let (_, mut tx) = self.stream.split();

        log::info!("[{id}] invalid token=\"{wrong}\"",);
        let _ = write_all(ctx, &mut tx, UNAUTHORIZED_RESPONSE.as_bytes()).await;
    }
}
