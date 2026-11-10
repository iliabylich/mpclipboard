use crate::{
    ServerContext,
    handshake::{Handshake, ID, Token},
    message::Message,
    message_reader::MessageReader,
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
use externally_driven_async_rt::net::{BorrowedReadHalf, BorrowedWriteHalf, TcpStream};

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

pub(crate) async fn client(
    ctx: ServerContext<'_>,
    mut stream: TcpStream,
    peer: SocketAddr,
    token: &Token,
    slots: &[RefCell<Slot>],
    current: &Cell<Option<Message>>,
) {
    let mut buf = [0; Message::BYTESIZE];
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

    let (mut rx, mut tx) = stream.split();
    serve(&ctx, &mut rx, &mut tx, &mut buf, &mut reader, id, &guard).await;
}

async fn serve(
    ctx: &ServerContext<'_>,
    rx: &mut BorrowedReadHalf<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    buf: &mut [u8; Message::BYTESIZE],
    reader: &mut MessageReader,
    id: ID,
    guard: &SlotGuard<'_>,
) {
    loop {
        let read = {
            let mut read = pin!(rx.read(ctx, buf));
            loop {
                let out = match select(read.as_mut(), guard.notified()).await {
                    Either::Left(read) => break read,
                    Either::Right(message) => message.encode(),
                };
                if write_all(ctx, tx, &out).await.is_err() {
                    log::error!("[{id}] failed to send a message");
                    return;
                }
            }
        };

        match read {
            Ok(0) => {
                log::info!("[{id}] disconnected");
                return;
            }
            Ok(len) => match reader.received(&buf[..len]) {
                Ok(Some(message)) => guard.publish(id, message),
                Ok(None) => {}
                Err(()) => {
                    log::error!("[{id}] received a malformed frame");
                    return;
                }
            },
            Err(err) => {
                log::error!("[{id}] read() failed: {err}");
                return;
            }
        }
    }
}
