use crate::{
    ServerContext,
    handshake::{Handshake, Token},
    message_reader::MessageReader,
    message_stream::MessageStream,
    pubsub::PubSubHandle,
    select::{Either, select},
    write_all::write_all,
};
use core::time::Duration;
use externally_driven_async_rt::net::TcpStream;

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

pub(crate) async fn client(
    ctx: ServerContext<'_>,
    mut stream: TcpStream,
    token: &Token,
    pubsub: PubSubHandle<'_>,
) {
    let mut reader = MessageReader::default();

    let handshake = Handshake::read_write(&ctx, &mut stream, token, &mut reader);
    let Some(handshake) = ctx.timeout(REAP_STRANGER_AFTER, handshake).await else {
        log::info!("no handshake within {}s", REAP_STRANGER_AFTER.as_secs());
        return;
    };
    let Ok((id, leftover_message)) = handshake else {
        return;
    };
    log::info!("[{id}] connected");

    if let Some(message) = leftover_message {
        pubsub.publish(id, message);
    }

    let (rx, mut tx) = stream.split();
    let mut messages = MessageStream::new(rx, reader, id);
    loop {
        match select(messages.next(&ctx), pubsub.receive()).await {
            Either::Left(Ok(message)) => pubsub.publish(id, message),
            Either::Left(Err(())) => return,
            Either::Right(message) => {
                if write_all(&ctx, &mut tx, &message.encode()).await.is_err() {
                    log::error!("[{id}] failed to send a message");
                    return;
                }
            }
        }
    }
}
