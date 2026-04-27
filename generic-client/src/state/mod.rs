mod connected;
mod connecting;
mod disconnected;
mod handshaking;
mod ready;

pub(crate) use connected::Connected;
pub(crate) use connecting::Connecting;
pub(crate) use disconnected::Disconnected;
pub(crate) use handshaking::Handshaking;
pub(crate) use ready::Ready;

use crate::{Connectivity, Context, Output, event_loop::EventLoop};
use clip::Clip;
use std::{os::fd::AsRawFd as _, rc::Rc};
use tungstenite::{Bytes, Message};

#[derive(Default)]
pub(crate) enum State {
    Connected(Connected),
    Connecting(Connecting),
    Handshaking(Handshaking),
    Ready(Ready),
    Disconnected(Disconnected),
    #[default]
    None,
}

macro_rules! for_each_variant {
    ($value:expr => |$var:ident| $eval:expr) => {
        match $value {
            Self::Connected($var) => $eval,
            Self::Connecting($var) => $eval,
            Self::Handshaking($var) => $eval,
            Self::Ready($var) => $eval,
            Self::Disconnected($var) => $eval,
            Self::None => panic!("bug: no state"),
        }
    };
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for_each_variant!(self => |inner| write!(f, "{}", inner.tag()))
    }
}

trait StateVariant {
    fn tag(&self) -> &'static str;
    fn context(&mut self) -> &mut Context;
    fn event_loop(&self) -> Rc<EventLoop>;

    fn transition(self, readable: bool, writable: bool) -> (State, Option<Output>);

    fn disconnect_by(context: Context, fd: i32) -> (State, Option<Output>) {
        context.event_loop.remove(fd);
        (
            State::Disconnected(Disconnected::new(context)),
            Some(Output::ConnectivityChanged {
                connectivity: Connectivity::Disconnected,
            }),
        )
    }
    fn flip(self) -> (State, Option<Output>);
}

impl State {
    pub(crate) fn start(context: Context) -> Self {
        let (this, _) = Disconnected::new(context).transition(true, true);
        this
    }

    pub(crate) fn tag(&self) -> &'static str {
        for_each_variant!(self => |inner| inner.tag())
    }

    pub(crate) fn context(&mut self) -> &mut Context {
        for_each_variant!(self => |inner| inner.context())
    }

    pub(crate) fn event_loop(&self) -> Rc<EventLoop> {
        for_each_variant!(self => |inner| inner.event_loop())
    }

    fn modify(&mut self, f: impl FnOnce(State) -> (State, Option<Output>)) -> Option<Output> {
        let this = std::mem::take(self);
        let before = this.tag();

        let (this, output) = f(this);

        let after = this.tag();
        if before != after {
            log::trace!("{before:?} -> {after:?}");
        }

        *self = this;
        output
    }

    pub(crate) fn transition(&mut self, readable: bool, writable: bool) -> Option<Output> {
        self.context().last_time_worked_with_ws_at = self.context().timer.now();

        self.modify(
            move |this| for_each_variant!(this => |inner| inner.transition(readable, writable)),
        )
    }

    pub(crate) fn tick(&mut self) -> Option<Output> {
        let last_time_worked_with_ws_at = self.context().last_time_worked_with_ws_at;
        let now = self.context().timer.now();

        log::trace!("state tick {last_time_worked_with_ws_at} <=> {now}");
        if (now - last_time_worked_with_ws_at) > 5 {
            self.context().last_time_worked_with_ws_at = now;
            self.modify(move |this| for_each_variant!(this => |inner| inner.flip()))
        } else {
            None
        }
    }

    pub(crate) fn push(&mut self, clip: Clip) -> bool {
        if !clip.newer_than(&self.context().last_clip) {
            return false;
        }
        self.context().last_clip = clip.clone();
        self.context().pending_message_to_send = Some(Message::Binary(Bytes::from(clip.encode())));

        if let State::Ready(ready) = &self {
            self.event_loop().modify(ready.as_raw_fd(), true, true);
        }

        true
    }
}
