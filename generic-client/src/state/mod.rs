mod disconnected;
mod established;
mod establishing;
mod handshaking;
mod ready;

use disconnected::Disconnected;
use established::Established;
use establishing::Establishing;
use handshaking::Handshaking;
use ready::Ready;

use crate::{Connectivity, Context, Output, event_loop::EventLoop};
use anyhow::Result;
use clip::Clip;
use std::{os::fd::AsRawFd, rc::Rc};
use tungstenite::{Bytes, Message};

pub(crate) enum Connected {
    Established(Established),
    Establishing(Establishing),
    Handshaking(Handshaking),
    Ready(Ready),
}
impl Connected {
    const fn tag(&self) -> &'static str {
        match self {
            Self::Established(_) => "Established",
            Self::Establishing(_) => "Establishing",
            Self::Handshaking(_) => "Handshaking",
            Self::Ready(_) => "Ready",
        }
    }
    fn try_transition(
        self,
        context: &mut Context,
        readable: bool,
        writable: bool,
    ) -> Result<(Self, Option<Output>)> {
        match self {
            Self::Established(s) => s.start_handshake(context),
            Self::Establishing(s) => s.finish_connecting(context),
            Self::Handshaking(s) => s.finish_handshake(context),
            Self::Ready(s) => s.read_write(context, readable, writable),
        }
    }
    fn disconnect(self, context: &Context) -> (Connection, Option<Output>) {
        context.event_loop.remove(self.as_raw_fd());
        let now = context.timer.now();
        (
            Connection::Disconnected(Disconnected::new(now)),
            Some(Output::ConnectivityChanged {
                connectivity: Connectivity::Disconnected,
            }),
        )
    }
    const fn should_disconnect_at(&self) -> u64 {
        match self {
            Self::Established(s) => s.should_disconnect_at(),
            Self::Establishing(s) => s.should_disconnect_at(),
            Self::Handshaking(s) => s.should_disconnect_at(),
            Self::Ready(s) => s.should_disconnect_at(),
        }
    }
}
impl AsRawFd for Connected {
    fn as_raw_fd(&self) -> i32 {
        match self {
            Self::Established(s) => s.as_raw_fd(),
            Self::Establishing(s) => s.as_raw_fd(),
            Self::Handshaking(s) => s.as_raw_fd(),
            Self::Ready(s) => s.as_raw_fd(),
        }
    }
}

pub(crate) enum Connection {
    Connected(Box<Connected>),
    Disconnected(Disconnected),
}
impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag())
    }
}
impl Connection {
    fn tag(&self) -> &'static str {
        match self {
            Self::Connected(s) => s.tag(),
            Self::Disconnected(_) => Disconnected::tag(),
        }
    }
    fn transition(
        self,
        readable: bool,
        writable: bool,
        context: &mut Context,
    ) -> (Self, Option<Output>) {
        let now = context.timer.now();

        match self {
            Self::Connected(connected) => {
                let fd = connected.as_raw_fd();

                match connected.try_transition(context, readable, writable) {
                    Ok((connected, output)) => (Self::Connected(Box::new(connected)), output),
                    Err(err) => {
                        log::error!("{err:?}");
                        context.event_loop.remove(fd);
                        (
                            Self::Disconnected(Disconnected::new(now)),
                            Some(Output::ConnectivityChanged {
                                connectivity: Connectivity::Disconnected,
                            }),
                        )
                    }
                }
            }

            Self::Disconnected(_) => Disconnected::connect(context),
        }
    }
    fn flip(self, context: &Context) -> (Self, Option<Output>) {
        match self {
            Self::Connected(connected) => connected.disconnect(context),
            Self::Disconnected(_) => Disconnected::connect(context),
        }
    }
    fn should_flip_at(&self) -> u64 {
        match self {
            Self::Connected(connected) => connected.should_disconnect_at(),
            Self::Disconnected(disconnected) => disconnected.should_reconnect_at(),
        }
    }
}

pub(crate) struct State {
    connection: Connection,
    context: Context,
    event_loop: Rc<EventLoop>,
}

impl State {
    pub(crate) fn start(context: Context, event_loop: Rc<EventLoop>) -> Self {
        let (connection, _) = Disconnected::connect(&context);
        Self {
            connection,
            context,
            event_loop,
        }
    }

    pub(crate) fn transition(&mut self, readable: bool, writable: bool) -> Option<Output> {
        let now = self.context.timer.now();

        let before = self.connection.tag();
        let mut connection = Connection::Disconnected(Disconnected::new(now));
        std::mem::swap(&mut self.connection, &mut connection);
        let (connection, output) = connection.transition(readable, writable, &mut self.context);
        self.connection = connection;
        let after = self.connection.tag();

        if before != after {
            log::trace!("{before} -> {after}");
        }

        output
    }

    pub(crate) fn tick(&mut self) -> Option<Output> {
        let should_flip_at = self.connection.should_flip_at();
        let now = self.context.timer.now();

        log::trace!("state tick {should_flip_at} <=> {now}");
        if now > should_flip_at {
            let before = self.connection.tag();
            let mut connection = Connection::Disconnected(Disconnected::new(0));
            std::mem::swap(&mut self.connection, &mut connection);
            let (connection, output) = connection.flip(&self.context);
            self.connection = connection;
            let after = self.connection.tag();

            if before != after {
                log::trace!("{before} -> {after}");
            }

            output
        } else {
            None
        }
    }

    pub(crate) fn push(&mut self, clip: Clip) -> Result<bool> {
        if !clip.newer_than(&self.context.last_clip) {
            return Ok(false);
        }
        self.context.last_clip = clip.clone();
        self.context.pending_message_to_send = Some(Message::Binary(Bytes::from(clip.encode())));

        if let Connection::Connected(connected) = &self.connection
            && let Connected::Ready(ready) = connected.as_ref()
        {
            self.event_loop.modify(ready.as_raw_fd(), true, true)?;
        }

        Ok(true)
    }
}
