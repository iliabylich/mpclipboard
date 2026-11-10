use crate::{
    ServerContext,
    message::Message,
    message_reader::MessageReader,
    request_reader::{self, ID, MAX_REQUEST_SIZE, Token},
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

    let guard = {
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
            guard.publish(message);
        }
        guard
    };

    let (mut rx, mut tx) = stream.split();
    serve(&ctx, &mut rx, &mut tx, &mut buf, &mut reader, &guard).await;
}

async fn handshake(
    ctx: &ServerContext<'_>,
    stream: &mut TcpStream,
    peer: SocketAddr,
    token: &Token,
    reader: &mut MessageReader,
) -> Result<(ID, Option<Message>), ()> {
    let mut buf = [0; MAX_REQUEST_SIZE];
    let mut len = 0;

    let (id, authorized, leftover) = {
        let (request, leftover) = loop {
            len += match stream.read(ctx, &mut buf[len..]).await {
                Ok(0) => {
                    log::info!("[{peer}] disconnected during handshake");
                    return Err(());
                }
                Ok(read) => read,
                Err(err) => {
                    log::error!("[{peer}] read() failed during handshake: {err}");
                    return Err(());
                }
            };

            match request_reader::parse(&buf[..len]) {
                Ok(None) => {}
                Ok(Some((request, consumed))) => break (request, consumed..len),
                Err(err) => {
                    log::error!("[{peer}] {err}");
                    return Err(());
                }
            }
        };

        let authorized = request.token == *token;
        if !authorized {
            log::info!("[{}] invalid token=\"{}\"", request.id, request.token);
        }
        (request.id, authorized, leftover)
    };

    let response = if authorized {
        SWITCHING_PROTOCOLS
    } else {
        UNAUTHORIZED
    };
    let (_, mut tx) = stream.split();
    let written = write_all(ctx, &mut tx, response).await;

    if !authorized {
        return Err(());
    }
    if written.is_err() {
        log::error!("[{id}] failed to send 101 Switching Protocols");
        return Err(());
    }

    let Ok(leftover_message) = reader.received(&buf[leftover]) else {
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
                    log::error!("[{}] failed to send a message", guard.id());
                    return;
                }
            }
        };

        match read {
            Ok(0) => {
                log::info!("[{}] disconnected", guard.id());
                return;
            }
            Ok(len) => match reader.received(&buf[..len]) {
                Ok(Some(message)) => guard.publish(message),
                Ok(None) => {}
                Err(()) => {
                    log::error!("[{}] received a malformed frame", guard.id());
                    return;
                }
            },
            Err(err) => {
                log::error!("[{}] read() failed: {err}", guard.id());
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
