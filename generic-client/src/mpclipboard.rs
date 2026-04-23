use crate::{
    Config, Context, Output,
    event_loop::EventLoop,
    logger::Logger,
    state::{Disconnected, State},
    tls::TLS,
};
use anyhow::Result;
use clip::Clip;
use std::{
    net::SocketAddr,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    rc::Rc,
};
use tungstenite::{Bytes, Message};

/// The main entrypoint
pub struct MPClipboard {
    config: Config,
    remote_addr: SocketAddr,
    tls: TLS,
    event_loop: Rc<EventLoop>,

    state: State,

    pending_message_to_send: Option<Message>,
    last_clip: Clip,
}

impl MPClipboard {
    /// Initializes MPClipboard, must be called once at the start of the program.
    /// Internally initializes logger and TLS.
    pub fn init() -> Result<()> {
        Logger::init();
        TLS::init()?;
        Ok(())
    }

    /// Constructs a new instance
    pub fn new(context: Context) -> Self {
        let Context {
            config,
            remote_addr,
            tls,
            event_loop,
        } = context;

        let (state, _output) = Disconnected::connect(remote_addr, Rc::clone(&event_loop));

        Self {
            config,
            remote_addr,
            tls,
            event_loop,

            state,

            pending_message_to_send: None,
            last_clip: Clip::zero(),
        }
    }

    /// Reads data from WebSocket connection, returns Output
    pub fn read(&mut self) -> Option<Output> {
        let event = self.event_loop.read();

        if let Some(tick) = event.tick {
            log::trace!("tick {tick}");

            if tick.is_multiple_of(5) && matches!(self.state, State::Disconnected(_)) {
                let (state, output) =
                    Disconnected::connect(self.remote_addr, Rc::clone(&self.event_loop));
                self.state = state;
                return output;
            }
        }

        if let Some((readable, writable)) = event.ws {
            let (state, output) = match &mut self.state {
                State::Connecting(connecting) => connecting.finish(),
                State::Connected(connected) => connected.start_handshake(&self.config, &self.tls),
                State::Handshaking(handshaking) => handshaking.finish_handshake(),
                State::Ready(ready) => ready.read_write(
                    readable,
                    writable,
                    &mut self.pending_message_to_send,
                    &mut self.last_clip,
                ),
                State::Disconnected(_) => {
                    unreachable!("bug: reading in Disconnected state");
                }
            };

            log::trace!("{:?} -> {:?}", self.state.tag(), state.tag());
            self.state = state;

            return output;
        }

        None
    }

    /// Pushes a new binary Clip with provided bytes.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    #[must_use]
    pub fn push_binary(&mut self, bytes: Vec<u8>) -> bool {
        self.push_clip(Clip::binary(bytes))
    }

    /// Pushes a new text Clip with provided content.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    #[must_use]
    pub fn push_text(&mut self, text: String) -> bool {
        self.push_clip(Clip::text(text))
    }

    #[must_use]
    fn push_clip(&mut self, clip: Clip) -> bool {
        if !clip.newer_than(&self.last_clip) {
            return false;
        }
        self.last_clip = clip.clone();

        self.pending_message_to_send = Some(Message::Binary(Bytes::from(clip.encode())));

        if let State::Ready(ready) = &self.state {
            self.event_loop.modify(ready.as_raw_fd(), true, true);
        }

        true
    }
}

impl AsRawFd for MPClipboard {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.event_loop.as_raw_fd()
    }
}

impl AsFd for MPClipboard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}
