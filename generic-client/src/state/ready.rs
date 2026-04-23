use crate::{
    event_loop::EventLoop,
    state::{Disconnected, State},
};
use clip::Clip;
use std::{io::ErrorKind, net::TcpStream, os::fd::AsRawFd, rc::Rc};
use tungstenite::{Message, stream::MaybeTlsStream};

/// cbindgen:ignore
type WebSocket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;

pub(crate) struct Ready {
    ws_and_fd: Option<(WebSocket, i32)>,
    event_loop: Rc<EventLoop>,
}

impl Ready {
    pub(crate) fn new(ws: WebSocket, fd: i32, event_loop: Rc<EventLoop>) -> Self {
        Self {
            ws_and_fd: Some((ws, fd)),
            event_loop,
        }
    }

    pub(crate) fn read_write(
        &mut self,
        readable: bool,
        writable: bool,
        pending_message_to_send: &mut Option<Message>,
        write_blocked: &mut bool,
    ) -> (State, Option<Clip>) {
        let (mut ws, rawfd) = self
            .ws_and_fd
            .take()
            .expect("bug: malformed state in Ready");

        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                self.event_loop.remove(rawfd);
                return (State::Disconnected(Disconnected), None);
            }};
        }

        if writable
            && ws.can_write()
            && let Some(message) = pending_message_to_send.take()
        {
            match ws.write(message) {
                Ok(()) => match ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                        *write_blocked = true
                    }
                    Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                        *pending_message_to_send = Some(*write_me_back);
                    }
                    Err(err) => disconnect!(err),
                },

                Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                    *write_blocked = true
                }
                Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                    *pending_message_to_send = Some(*write_me_back);
                }
                Err(err) => disconnect!(err),
            };
        }

        let mut clip = None;

        if readable && ws.can_read() {
            match ws.read() {
                Ok(message) => {
                    log::trace!("{message:?}");
                    if let Message::Binary(bytes) = message {
                        match Clip::decode(bytes.into()) {
                            Ok(decoded) => clip = Some(decoded),
                            Err(err) => disconnect!(err),
                        }
                    }
                }
                Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                    log::trace!("nothing to read");
                }
                Err(err) => disconnect!(err),
            };
        }

        (
            State::Ready(Ready::new(ws, rawfd, Rc::clone(&self.event_loop))),
            clip,
        )
    }
}

impl AsRawFd for Ready {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.ws_and_fd.as_ref().unwrap().1
    }
}
