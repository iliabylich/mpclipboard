use crate::{
    ServerContext,
    handshake::{Handshake, ID, Token},
    message::Message,
    message_reader::MessageReader,
    select::{Either, select},
    slot::{Slot, SlotGuard},
};
use core::{
    cell::{Cell, RefCell},
    net::SocketAddr,
    pin::pin,
    time::Duration,
};
use externally_driven_async_rt::net::{BorrowedReadHalf, BorrowedWriteHalf, TcpStream};

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

const SWITCHING_PROTOCOLS: &[u8] =
    b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n";
const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

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
        let handshake = handshake(&ctx, &mut stream, peer, token, &mut reader);
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

async fn handshake(
    ctx: &ServerContext<'_>,
    stream: &mut TcpStream,
    peer: SocketAddr,
    token: &Token,
    reader: &mut MessageReader,
) -> Result<(ID, Option<Message>), ()> {
    let mut handshake = Handshake::default();
    let (received_token, id, leftover) = handshake.read(ctx, stream, peer).await?;

    let (_, mut tx) = stream.split();

    if received_token != *token {
        log::info!("[{id}] invalid token=\"{received_token}\"");
        let _ = write_all(ctx, &mut tx, UNAUTHORIZED).await;
        return Err(());
    }

    if write_all(ctx, &mut tx, SWITCHING_PROTOCOLS).await.is_err() {
        log::error!("[{id}] failed to send 101 Switching Protocols");
        return Err(());
    }

    let Ok(leftover_message) = reader.received(leftover) else {
        log::error!("[{id}] sent a malformed frame right after the request");
        return Err(());
    };
    Ok((id, leftover_message))
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
                let out = match select(read.as_mut(), pin!(guard.notified())).await {
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

async fn write_all(
    ctx: &ServerContext<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    mut buf: &[u8],
) -> Result<(), ()> {
    while !buf.is_empty() {
        match tx.write(ctx, buf).await {
            Ok(0) => {
                log::error!("write() wrote 0 bytes");
                return Err(());
            }
            Ok(len) => buf = &buf[len..],
            Err(err) => {
                log::error!("write() failed: {err}");
                return Err(());
            }
        }
    }
    Ok(())
}
