use crate::{ServerContext, errors::ClientError, pubsub::PubSubHandle};
use externally_driven_async_rt::{Either, io::AsyncWriteExt, net::TcpStream, select};
use mpclipboard_shared2::{ID, MessageReader};

pub(crate) struct Client<'a> {
    stream: TcpStream,
    id: ID,
    pubsub: PubSubHandle<'a>,
}

impl<'a> Client<'a> {
    pub(crate) fn new(stream: TcpStream, id: ID, pubsub: PubSubHandle<'a>) -> Self {
        Self { stream, id, pubsub }
    }

    pub(crate) async fn read_write_messages(
        self,
        ctx: &ServerContext<'_>,
    ) -> Result<(), ClientError> {
        let Self {
            mut stream,
            id,
            pubsub,
        } = self;

        let (rx, mut tx) = stream.split();
        let mut reader = MessageReader::new(rx);
        loop {
            match select(reader.read(ctx), pubsub.receive()).await {
                Either::Left(Ok(Some(message))) => pubsub.publish(id, message),
                Either::Left(Ok(None)) => {
                    info!("[{id}] disconnected");
                    return Ok(());
                }
                Either::Left(Err(err)) => return Err(ClientError::Read { id, err }),
                Either::Right(message) => tx
                    .write_all(ctx, &message.encode())
                    .await
                    .map_err(|err| ClientError::Write { id, err })?,
            }
        }
    }
}
