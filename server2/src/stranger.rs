use crate::{
    ServerContext,
    client::Client,
    constants::{MAX_REQUEST_SIZE, SWITCHING_PROTOCOLS, UNAUTHORIZED},
    handshake::{ID, Token, parse},
    message::Message,
    message_reader::MessageReader,
    pubsub::PubSubHandle,
    write_all::write_all,
};
use core::time::Duration;
use externally_driven_async_rt::net::TcpStream;

pub(crate) struct Stranger<'a> {
    stream: TcpStream,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
    buf: [u8; MAX_REQUEST_SIZE],
    len: usize,
}

impl<'a> Stranger<'a> {
    pub(crate) fn new(stream: TcpStream, token: &'a Token, pubsub: PubSubHandle<'a>) -> Self {
        Self {
            stream,
            token,
            pubsub,
            buf: [0; MAX_REQUEST_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn handshake(
        mut self,
        ctx: &ServerContext<'_>,
        timeout: Duration,
    ) -> Result<Client<'a>, ()> {
        let mut reader = MessageReader::default();

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

        let (_, mut tx) = self.stream.split();

        if received_token != *self.token {
            log::info!("[{id}] invalid token=\"{received_token}\"");
            let _ = write_all(ctx, &mut tx, UNAUTHORIZED).await;
            return Err(());
        }

        if write_all(ctx, &mut tx, SWITCHING_PROTOCOLS).await.is_err() {
            log::error!("[{id}] failed to send 101 Switching Protocols");
            return Err(());
        }

        let Ok(leftover_message) = reader.received(&self.buf[consumed..self.len]) else {
            log::error!("[{id}] sent a malformed frame right after the request");
            return Err(());
        };
        Ok((id, leftover_message))
    }

    async fn read(&mut self, ctx: &ServerContext<'_>) -> Result<(Token, ID, usize), ()> {
        loop {
            self.len += match self.stream.read(ctx, &mut self.buf[self.len..]).await {
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
                Ok(Some(parsed)) => return Ok(parsed),
                Err(err) => {
                    log::error!("{err}");
                    return Err(());
                }
            }
        }
    }
}
