use crate::{
    ServerContext,
    handshake::{Handshake, ID, Token},
    message::Message,
    message_reader::MessageReader,
    message_stream::MessageStream,
    select::{Either, select},
    slot::{Slot, SlotGuard},
    write_all::write_all,
};
use core::{
    cell::{Cell, RefCell},
    net::SocketAddr,
    pin::pin,
    time::Duration,
};
use externally_driven_async_rt::net::{BorrowedWriteHalf, TcpStream};

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

pub(crate) async fn client(
    ctx: ServerContext<'_>,
    mut stream: TcpStream,
    peer: SocketAddr,
    token: &Token,
    slots: &[RefCell<Slot>],
    current: &Cell<Option<Message>>,
) {
    let mut reader = MessageReader::default();

    let (id, guard) = {
        let handshake = Handshake::read_write(&ctx, &mut stream, peer, token, &mut reader);
        let Some(handshake) = ctx.timeout(REAP_STRANGER_AFTER, handshake).await else {
            log::info!(
                "[{peer}] no handshake within {}s",
                REAP_STRANGER_AFTER.as_secs()
            );
            return;
        };
        let Ok((id, leftover_message)) = handshake else {
            return;
        };

        let Ok(guard) = SlotGuard::take(slots, current, id) else {
            return;
        };
        log::info!("[{id}] connected from {peer}");

        if let Some(message) = leftover_message {
            guard.publish(id, message);
        }
        (id, guard)
    };

    let (rx, mut tx) = stream.split();
    let mut messages = MessageStream::new(rx, reader, id);
    serve(&ctx, &mut messages, &mut tx, id, &guard).await;
}

async fn serve(
    ctx: &ServerContext<'_>,
    messages: &mut MessageStream<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    id: ID,
    guard: &SlotGuard<'_>,
) {
    loop {
        let next = {
            let mut next = pin!(messages.next(ctx));
            loop {
                match select(next.as_mut(), guard.notified()).await {
                    Either::Left(next) => break next,
                    Either::Right(message) => {
                        if write_all(ctx, tx, &message.encode()).await.is_err() {
                            log::error!("[{id}] failed to send a message");
                            return;
                        }
                    }
                }
            }
        };

        let Ok(Some(message)) = next else {
            return;
        };
        guard.publish(id, message);
    }
}
