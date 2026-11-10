use crate::{
    ServerContext,
    id::ID,
    message_reader::MessageReader,
    message_stream::MessageStream,
    pubsub::PubSubHandle,
    select::{Either, select},
    write_all::write_all,
};
use externally_driven_async_rt::net::TcpStream;

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

    pub(crate) async fn read_write_messages(self, ctx: &ServerContext<'_>) {
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
                Either::Left(Ok(message)) => pubsub.publish(id, message),
                Either::Left(Err(())) => return,
                Either::Right(message) => {
                    if write_all(ctx, &mut tx, &message.encode()).await.is_err() {
                        log::error!("[{id}] failed to send a message");
                        return;
                    }
                }
            }
        }
    }
}
