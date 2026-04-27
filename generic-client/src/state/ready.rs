use crate::{
    Context, Output,
    event_loop::EventLoop,
    state::{State, StateVariant},
};
use clip::Clip;
use std::{io::ErrorKind, net::TcpStream, os::fd::AsRawFd, rc::Rc};
use tungstenite::{Message, stream::MaybeTlsStream};

/// cbindgen:ignore
type WebSocket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;

pub(crate) struct Ready {
    ws: WebSocket,
    fd: i32,
    context: Context,
}

impl Ready {
    pub(crate) fn new(ws: WebSocket, fd: i32, context: Context) -> Self {
        Self { ws, fd, context }
    }

    fn read_write(mut self, readable: bool, writable: bool) -> (State, Option<Output>) {
        macro_rules! disconnect {
            ($err:expr) => {{
                log::error!("{:?}", $err);
                return self.disconnect();
            }};
        }

        let mut write_blocked = false;

        if writable
            && self.ws.can_write()
            && let Some(message) = self.context.pending_message_to_send.take()
        {
            match self.ws.write(message) {
                Ok(()) => match self.ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                        write_blocked = true
                    }
                    Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                        self.context.pending_message_to_send = Some(*write_me_back);
                    }
                    Err(err) => disconnect!(err),
                },

                Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                    write_blocked = true
                }
                Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                    self.context.pending_message_to_send = Some(*write_me_back);
                }
                Err(err) => disconnect!(err),
            };
        }

        let mut clip = None;

        if readable && self.ws.can_read() {
            match self.ws.read() {
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

        let wants_write = write_blocked || self.context.pending_message_to_send.is_some();
        self.context.event_loop.modify(self.fd, true, wants_write);

        let output = if let Some(clip) = clip
            && clip.newer_than(&self.context.last_clip)
        {
            self.context.last_clip = clip.clone();

            Some(Output::NewText { text: clip.text })
        } else {
            None
        };

        (
            State::Ready(Ready::new(self.ws, self.fd, self.context)),
            output,
        )
    }

    fn disconnect(self) -> (State, Option<Output>) {
        Self::disconnect_by(self.context, self.fd)
    }
}

impl AsRawFd for Ready {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd
    }
}

impl StateVariant for Ready {
    fn tag(&self) -> &'static str {
        "Ready"
    }

    fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.context.event_loop)
    }

    fn transition(self, readable: bool, writable: bool) -> (State, Option<Output>) {
        self.read_write(readable, writable)
    }

    fn flip(self) -> (State, Option<Output>) {
        self.disconnect()
    }
}
