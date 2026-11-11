use crate::{
    ServerContext, errors::ClientError, id::ID, message_reader::MessageReader,
    message_stream::MessageStream, pubsub::PubSubHandle,
};
use externally_driven_async_rt::{Either, io::AsyncWriteExt, net::TcpStream, select};

pub(crate) struct Client<'a> {
    stream: TcpStream,
    reader: MessageReader,
    id: ID,
    pubsub: PubSubHandle<'a>,
}

impl<'a> Client<'a> {
    pub(crate) fn new(
        stream: TcpStream,
        reader: MessageReader,
        id: ID,
        pubsub: PubSubHandle<'a>,
    ) -> Self {
        Self {
            stream,
            reader,
            id,
            pubsub,
        }
    }

    pub(crate) async fn read_write_messages(
        self,
        ctx: &ServerContext<'_>,
    ) -> Result<(), ClientError> {
        let Self {
            mut stream,
            reader,
            id,
            pubsub,
        } = self;

        let (rx, mut tx) = stream.split();
        let mut messages = MessageStream::new(rx, reader, id);
        loop {
            match select(messages.next(ctx), pubsub.receive()).await {
                Either::Left(Ok(Some(message))) => pubsub.publish(id, message),
                Either::Left(Ok(None)) => return Ok(()),
                Either::Left(Err(err)) => return Err(err),
                Either::Right(message) => tx
                    .write_all(ctx, &message.encode())
                    .await
                    .map_err(|err| ClientError::Write { id, err })?,
            }
        }
    }
}
