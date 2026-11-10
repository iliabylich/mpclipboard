use crate::{
    ServerContext,
    message::Message,
    message_reader::MessageReader,
    request_reader::{ID, RequestReader, Token},
    select::{Either, select},
    slot::{Slot, SlotGuard},
};
use core::{
    cell::{Cell, RefCell, RefMut},
    net::SocketAddr,
    pin::pin,
    time::Duration,
};
use externally_driven_async_rt::net::{BorrowedReadHalf, BorrowedWriteHalf, TcpStream};

const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

const SWITCHING_PROTOCOLS: &[u8] =
    b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\n\r\n";
const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

pub(crate) struct ClientState {
    buf: [u8; Message::BYTESIZE],
    out: [u8; Message::BYTESIZE],
    request_reader: RequestReader,
    reader: MessageReader,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            buf: [0; Message::BYTESIZE],
            out: [0; Message::BYTESIZE],
            request_reader: RequestReader::default(),
            reader: MessageReader::default(),
        }
    }
}

#[expect(clippy::await_holding_refcell_ref)]
pub(crate) async fn client(
    ctx: ServerContext<'_>,
    mut stream: TcpStream,
    peer: SocketAddr,
    token: &Token,
    slots: &[RefCell<Slot>],
    current: &Cell<Option<Message>>,
    mut state: RefMut<'_, ClientState>,
) {
    *state = ClientState::default();
    let state = &mut *state;

    let (mut rx, mut tx) = stream.split();

    let guard = {
        let handshake = handshake(&ctx, &mut rx, &mut tx, peer, token, state);
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

    serve(&ctx, &mut rx, &mut tx, state, &guard).await;
}

async fn handshake(
    ctx: &ServerContext<'_>,
    rx: &mut BorrowedReadHalf<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    peer: SocketAddr,
    token: &Token,
    state: &mut ClientState,
) -> Result<(ID, Option<Message>), ()> {
    let ClientState {
        buf,
        request_reader,
        reader,
        ..
    } = state;

    let (request, leftover) = {
        loop {
            let len = match rx.read(ctx, buf).await {
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
                Ok(Some((request, consumed))) => break (request, consumed..len),
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
        log::info!("[{}] invalid token=\"{}\"", request.id, request.token);
        UNAUTHORIZED
    };
    let written = write_all(ctx, tx, response).await;

    if !authorized {
        return Err(());
    }
    if written.is_err() {
        log::error!("[{}] failed to send 101 Switching Protocols", request.id);
        return Err(());
    }

    let Ok(leftover_message) = reader.received(&buf[leftover]) else {
        log::error!(
            "[{}] sent a malformed frame right after the request",
            request.id
        );
        return Err(());
    };
    Ok((request.id, leftover_message))
}

async fn serve(
    ctx: &ServerContext<'_>,
    rx: &mut BorrowedReadHalf<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    state: &mut ClientState,
    guard: &SlotGuard<'_>,
) {
    let ClientState {
        buf, out, reader, ..
    } = state;

    loop {
        let read = {
            let mut read = pin!(rx.read(ctx, buf));
            loop {
                match select(read.as_mut(), pin!(guard.notified())).await {
                    Either::Left(read) => break read,
                    Either::Right(message) => *out = message.encode(),
                }
                if write_all(ctx, tx, out).await.is_err() {
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
