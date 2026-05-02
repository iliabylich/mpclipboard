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
use anyhow::{Context as _, Result};
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
            Self::Handshaking(s) => s.finish_handshake(),
            Self::Ready(s) => s.read_write(context, readable, writable),
        }
    }
    fn disconnect(self, context: &Context) -> (Connection, Option<Output>) {
        context.event_loop.remove(self.as_raw_fd());
        (
            Connection::Disconnected,
            Some(Output::ConnectivityChanged {
                connectivity: Connectivity::Disconnected,
            }),
        )
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

#[derive(Default)]
pub(crate) enum Connection {
    Connected(Box<Connected>),
    #[default]
    Disconnected,
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
            Self::Disconnected => Disconnected::tag(),
        }
    }
    fn transition(
        self,
        readable: bool,
        writable: bool,
        context: &mut Context,
    ) -> (Self, Option<Output>) {
        match self {
            Self::Connected(connected) => {
                let fd = connected.as_raw_fd();

                match connected.try_transition(context, readable, writable) {
                    Ok((connected, output)) => (Self::Connected(Box::new(connected)), output),
                    Err(err) => {
                        log::error!("{err:?}");
                        context.event_loop.remove(fd);
                        (
                            Self::Disconnected,
                            Some(Output::ConnectivityChanged {
                                connectivity: Connectivity::Disconnected,
                            }),
                        )
                    }
                }
            }

            Self::Disconnected => Disconnected::connect(context),
        }
    }
    fn flip(self, context: &Context) -> (Self, Option<Output>) {
        match self {
            Self::Connected(connected) => connected.disconnect(context),
            Self::Disconnected => Disconnected::connect(context),
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
        self.context.last_time_worked_with_ws_at = self.context.timer.now();

        let before = self.connection.tag();
        let connection = std::mem::take(&mut self.connection);
        let (connection, output) = connection.transition(readable, writable, &mut self.context);
        self.connection = connection;
        let after = self.connection.tag();

        if before != after {
            log::trace!("{before} -> {after}");
        }

        output
    }

    pub(crate) fn tick(&mut self) -> Result<Option<Output>> {
        let last_time_worked_with_ws_at = self.context.last_time_worked_with_ws_at;
        let now = self.context.timer.now();

        log::trace!("state tick {last_time_worked_with_ws_at} <=> {now}");
        if now
            .checked_sub(last_time_worked_with_ws_at)
            .context("time goes backwards")?
            > 5
        {
            self.context.last_time_worked_with_ws_at = now;

            let before = self.connection.tag();
            let connection = std::mem::take(&mut self.connection);
            let (connection, output) = connection.flip(&self.context);
            self.connection = connection;
            let after = self.connection.tag();

            if before != after {
                log::trace!("{before} -> {after}");
            }

            Ok(output)
        } else {
            Ok(None)
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
