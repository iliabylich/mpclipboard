use crate::{
    ServerContext, client::Client, constants::HANDSHAKE_REQUEST_BUFFER_SIZE, errors::ClientError,
    pubsub::PubSubHandle,
};
use core::time::Duration;
use externally_driven_async_rt::{
    io::{AsyncRead, AsyncWriteExt},
    net::TcpStream,
};
use mpclipboard_shared2::{ID, Token};

pub(crate) struct Stranger<'a> {
    stream: TcpStream,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
    buf: [u8; HANDSHAKE_REQUEST_BUFFER_SIZE],
    len: usize,
}

impl<'a> Stranger<'a> {
    pub(crate) fn new(stream: TcpStream, token: &'a Token, pubsub: PubSubHandle<'a>) -> Self {
        Self {
            stream,
            token,
            pubsub,
            buf: [0; HANDSHAKE_REQUEST_BUFFER_SIZE],
            len: 0,
        }
    }

    pub(crate) async fn handshake(
        mut self,
        ctx: &ServerContext<'_>,
        timeout: Duration,
    ) -> Result<Option<Client<'a>>, ClientError> {
        let read_write = self.read_write(ctx);
        let Some(read_write) = ctx.timeout(timeout, read_write).await else {
            return Err(ClientError::HandshakeTimeout(timeout));
        };
        let Some(id) = read_write? else {
            return Ok(None);
        };
        info!("[{id}] connected");

        Ok(Some(Client::new(self.stream, id, self.pubsub)))
    }

    async fn read_write(&mut self, ctx: &ServerContext<'_>) -> Result<Option<ID>, ClientError> {
        let Some((received, id, consumed)) = self.read(ctx).await? else {
            return Ok(None);
        };

        if *self.token != received {
            self.write_401(ctx).await;
            return Err(ClientError::InvalidToken { id });
        }

        if consumed != self.len {
            return Err(ClientError::BytesAfterHandshake { id });
        }

        self.write_101(ctx, id).await?;

        Ok(Some(id))
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
        tx.write_all(ctx, SWITCHING_PROTOCOLS_RESPONSE.as_bytes())
            .await
            .map_err(|err| ClientError::Write { id, err })
    }

    async fn write_401(&mut self, ctx: &ServerContext<'_>) {
        const UNAUTHORIZED_RESPONSE: &str = concat!(
            "HTTP/1.1 401 Unauthorized\r\n",
            "Content-Length: 0\r\n",
            "\r\n",
        );

        let (_, mut tx) = self.stream.split();

        let _ = tx.write_all(ctx, UNAUTHORIZED_RESPONSE.as_bytes()).await;
    }
}
