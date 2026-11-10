use crate::{
    ServerContext, client::Client, constants::HANDSHAKE_REQUEST_SIZE, errors::ClientError, id::ID,
    message::Message, message_reader::MessageReader, pubsub::PubSubHandle, token::Token,
    write_all::write_all,
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
    ) -> Result<Option<Client<'a>>, ClientError> {
        let mut reader = MessageReader::new();

        let read_write = self.read_write(ctx, &mut reader);
        let Some(read_write) = ctx.timeout(timeout, read_write).await else {
            return Err(ClientError::HandshakeTimeout(timeout));
        };
        let Some((id, leftover_message)) = read_write? else {
            return Ok(None);
        };
        info!("[{id}] connected");

        if let Some(message) = leftover_message {
            self.pubsub.publish(id, message);
        }

        Ok(Some(Client::new(self.stream, reader, id, self.pubsub)))
    }

    async fn read_write(
        &mut self,
        ctx: &ServerContext<'_>,
        reader: &mut MessageReader,
    ) -> Result<Option<(ID, Option<Message>)>, ClientError> {
        let Some((received, id, consumed)) = self.read(ctx).await? else {
            return Ok(None);
        };

        if *self.token != received {
            self.write_401(ctx, id).await;
            return Err(ClientError::InvalidToken { id });
        }

        self.write_101(ctx, id).await?;

        let leftover_message = reader.received(&self.buf[consumed..self.len]);
        let leftover_message =
            leftover_message.map_err(|err| ClientError::MalformedFrame { id, err })?;
        Ok(Some((id, leftover_message)))
    }

    async fn read(
        &mut self,
        ctx: &ServerContext<'_>,
    ) -> Result<Option<(Token, ID, usize)>, ClientError> {
        loop {
            match self.stream.read(ctx, &mut self.buf[self.len..]).await {
                Ok(0) => {
                    info!("disconnected during handshake");
                    return Ok(None);
                }
                Ok(read) => {
                    self.len += read;
                }
                Err(err) => return Err(ClientError::HandshakeRead(err)),
            };

            if let Some(parsed) = crate::handshake::parse(&self.buf[..self.len])? {
                return Ok(Some(parsed));
            }
        }
    }

    async fn write_101(&mut self, ctx: &ServerContext<'_>, id: ID) -> Result<(), ClientError> {
        const SWITCHING_PROTOCOLS_RESPONSE: &str = concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "\r\n",
        );

        let (_, mut tx) = self.stream.split();
        write_all(ctx, &mut tx, SWITCHING_PROTOCOLS_RESPONSE.as_bytes(), id).await
    }

    async fn write_401(&mut self, ctx: &ServerContext<'_>, id: ID) {
        const UNAUTHORIZED_RESPONSE: &str = concat!(
            "HTTP/1.1 401 Unauthorized\r\n",
            "Content-Length: 0\r\n",
            "\r\n",
        );

        let (_, mut tx) = self.stream.split();

        let _ = write_all(ctx, &mut tx, UNAUTHORIZED_RESPONSE.as_bytes(), id).await;
    }
}
