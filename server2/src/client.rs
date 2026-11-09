use crate::{
    ServerContext,
    message::Message,
    message_reader::MessageReader,
    request_reader::{ID, RequestReader, Token},
    select::{Either, select},
    slot::{Slot, SlotGuard},
};
use core::{
    cell::{Cell, RefCell},
    net::SocketAddr,
    time::Duration,
};
use externally_driven_async_rt::net::TcpStream;

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

const SWITCHING_PROTOCOLS: &[u8] =
    b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n";
const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

pub(crate) async fn client(
    ctx: &ServerContext<'_>,
    mut stream: TcpStream,
    peer: SocketAddr,
    token: &Token,
    slots: &[RefCell<Slot>],
    current: &Cell<Option<Message>>,
) {
    let handshake = handshake(ctx, &mut stream, peer, token);
    let Some(handshake) = ctx.timeout(REAP_STRANGER_AFTER, handshake).await else {
        log::info!("[{peer}] no handshake within {REAP_STRANGER_AFTER:?}");
        return;
    };
    let Ok((id, reader, leftover_message)) = handshake else {
        return;
    };

    let Ok(guard) = SlotGuard::take(slots, current, id) else {
        return;
    };
    log::info!("[{id}] connected from {peer}");

    if let Some(message) = leftover_message {
        guard.publish(message);
    }

    serve(ctx, stream, id, reader, guard).await;
}

async fn handshake(
    ctx: &ServerContext<'_>,
    stream: &mut TcpStream,
    peer: SocketAddr,
    token: &Token,
) -> Result<(ID, MessageReader, Option<Message>), ()> {
    let (request, reader, leftover_message) = {
        let mut request_reader = RequestReader::default();
        let mut buf = [0; Message::BYTESIZE];
        loop {
            let len = match stream.read(ctx, &mut buf).await {
                Ok(0) => {
                    log::info!("[{peer}] disconnected during handshake");
                    return Err(());
                }
                Ok(len) => len,
                Err(err) => {
                    log::error!("[{peer}] read() failed during handshake: {err}");
                    return Err(());
                }
            };

            match request_reader.received(&buf[..len]) {
                Ok(None) => {}
                Ok(Some((request, consumed))) => {
                    let mut reader = MessageReader::default();
                    let Ok(leftover_message) = reader.received(&buf[consumed..len]) else {
                        log::error!("[{peer}] sent a malformed frame right after the request");
                        return Err(());
                    };
                    break (request, reader, leftover_message);
                }
                Err(err) => {
                    log::error!("[{peer}] {err}");
                    return Err(());
                }
            }
        }
    };

    let authorized = request.token == *token;
    let response = if authorized {
        SWITCHING_PROTOCOLS
    } else {
        log::info!("[{}] invalid token={:?}", request.id, request.token);
        UNAUTHORIZED
    };
    let written = write_all(ctx, stream, response).await;

    if !authorized {
        return Err(());
    }
    if written.is_err() {
        log::error!("[{}] failed to send 101 Switching Protocols", request.id);
        return Err(());
    }

    Ok((request.id, reader, leftover_message))
}

async fn serve(
    ctx: &ServerContext<'_>,
    mut stream: TcpStream,
    id: ID,
    mut reader: MessageReader,
    guard: SlotGuard<'_>,
) {
    let mut buf = [0; Message::BYTESIZE];
    loop {
        match select(stream.read(ctx, &mut buf), guard.notified()).await {
            Either::Left(Ok(0)) => {
                log::info!("[{id}] disconnected");
                return;
            }
            Either::Left(Ok(len)) => match reader.received(&buf[..len]) {
                Ok(Some(message)) => guard.publish(message),
                Ok(None) => {}
                Err(()) => {
                    log::error!("[{id}] received a malformed frame");
                    return;
                }
            },
            Either::Left(Err(err)) => {
                log::error!("[{id}] read() failed: {err}");
                return;
            }
            Either::Right(message) => {
                if write_all(ctx, &mut stream, &message.encode())
                    .await
                    .is_err()
                {
                    log::error!("[{id}] failed to send {message:?}");
                    return;
                }
            }
        }
    }
}

async fn write_all(
    ctx: &ServerContext<'_>,
    stream: &mut TcpStream,
    mut buf: &[u8],
) -> Result<(), ()> {
    while !buf.is_empty() {
        match stream.write(ctx, buf).await {
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
