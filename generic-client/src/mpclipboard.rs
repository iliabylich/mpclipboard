use crate::{
    Config, Connectivity, Context, Output,
    event_loop::EventLoop,
    logger::Logger,
    state::{Disconnected, State, StateTag},
    tls::TLS,
};
use anyhow::Result;
use clip::{Clip, TextOrBinary};
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
    last_received_clip_ts: u128,
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

        let state = Disconnected::connect(remote_addr, Rc::clone(&event_loop));

        Self {
            config,
            remote_addr,
            tls,
            event_loop,

            state,

            pending_message_to_send: None,
            last_received_clip_ts: 0,
        }
    }

    fn connectivity(&self) -> Connectivity {
        Connectivity::from(&self.state)
    }

    fn set_state(
        &mut self,
        state: State,
        state_was: StateTag,
        connectivity_was: Connectivity,
    ) -> Option<Connectivity> {
        self.state = state;

        let state_now = self.state.tag();
        if state_was != state_now {
            log::trace!("state changed: {state_was:?} -> {state_now:?}");
        }

        let connectivity_now = self.connectivity();
        if connectivity_was != connectivity_now {
            log::trace!("connectivity changed: {connectivity_was:?} -> {connectivity_now:?}");
            return Some(connectivity_now);
        }

        None
    }

    fn disconnect(&mut self) -> Option<Connectivity> {
        self.set_state(
            State::Disconnected(Disconnected),
            self.state.tag(),
            self.connectivity(),
        )
    }
    fn reconnect(&mut self) -> Option<Connectivity> {
        self.set_state(
            Disconnected::connect(self.remote_addr, Rc::clone(&self.event_loop)),
            self.state.tag(),
            self.connectivity(),
        )
    }

    /// Reads data from WebSocket connection, returns Output
    pub fn read(&mut self) -> Option<Output> {
        assert!(self.state.tag() != StateTag::Taken);

        let state_was = self.state.tag();
        let connectivity_was = self.connectivity();

        let event = match self.event_loop.read() {
            Ok(event) => event,
            Err(err) => {
                log::error!("{err:?}");
                return self
                    .disconnect()
                    .map(|connectivity| Output::ConnectivityChanged { connectivity });
            }
        };

        if let Some(tick) = event.tick {
            log::trace!("tick {tick}");

            if tick.is_multiple_of(5) && matches!(self.state, State::Disconnected(_)) {
                return self
                    .reconnect()
                    .map(|connectivity| Output::ConnectivityChanged { connectivity });
            }
        }

        if let Some((readable, writable)) = event.ws {
            let output = match std::mem::take(&mut self.state) {
                State::Connecting(connecting) => {
                    let state = connecting.finish();
                    let output = self
                        .set_state(state, state_was, connectivity_was)
                        .map(|connectivity| Output::ConnectivityChanged { connectivity });
                    self.update_fd_interest(false);
                    output
                }
                State::Connected(connected) => {
                    let state = connected.start_handshake(&self.config, &self.tls);
                    let output = self
                        .set_state(state, state_was, connectivity_was)
                        .map(|connectivity| Output::ConnectivityChanged { connectivity });
                    self.update_fd_interest(false);
                    output
                }
                State::Handshaking(handshaking) => {
                    let state = handshaking.finish_handshake();
                    let output = self
                        .set_state(state, state_was, connectivity_was)
                        .map(|connectivity| Output::ConnectivityChanged { connectivity });
                    self.update_fd_interest(false);
                    output
                }
                State::Ready(ready) => {
                    let mut write_blocked = false;

                    let (state, clip) = ready.read_write(
                        readable,
                        writable,
                        &mut self.pending_message_to_send,
                        &mut write_blocked,
                    );

                    let connectivity = self.set_state(state, state_was, connectivity_was);
                    self.update_fd_interest(write_blocked);

                    if let Some(clip) = clip
                        && clip.timestamp > self.last_received_clip_ts
                    {
                        self.last_received_clip_ts = clip.timestamp;

                        match (clip.text_or_binary, connectivity) {
                            (TextOrBinary::Text(text), None) => Some(Output::NewText { text }),
                            (TextOrBinary::Text(text), Some(connectivity)) => {
                                Some(Output::NewTextAndConnectivityChanged { text, connectivity })
                            }
                            (TextOrBinary::Binary(bytes), None) => {
                                Some(Output::NewBinary { bytes })
                            }
                            (TextOrBinary::Binary(bytes), Some(connectivity)) => {
                                Some(Output::NewBinaryAndConnectivityChanged {
                                    bytes,
                                    connectivity,
                                })
                            }
                        }
                    } else {
                        connectivity
                            .map(|connectivity| Output::ConnectivityChanged { connectivity })
                    }
                }
                State::Disconnected(_) => {
                    unreachable!("bug: reading in Disconnected state");
                }
                State::Taken => {
                    unreachable!("bug: reading in Taken state");
                }
            };

            assert!(!matches!(self.state, State::Taken));
            return output;
        }

        None
    }

    fn update_fd_interest(&mut self, write_blocked: bool) {
        let has_pending_message = self.pending_message_to_send.is_some();

        let Some((readable, writable, fd)) =
            self.state.interests(write_blocked, has_pending_message)
        else {
            return;
        };

        if let Err(err) = self.event_loop.modify(fd, readable, writable) {
            log::error!("{err:?}");
            self.disconnect();
        }
    }

    /// Pushes a new binary Clip with provided bytes.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    pub fn push_binary(&mut self, bytes: Vec<u8>) {
        let clip = Clip::binary(bytes);
        self.pending_message_to_send = Some(Message::Binary(Bytes::from(clip.encode())));
        self.update_fd_interest(false);
    }

    /// Pushes a new text Clip with provided content.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    pub fn push_text(&mut self, text: String) {
        let clip = Clip::text(text);
        self.pending_message_to_send = Some(Message::Binary(Bytes::from(clip.encode())));
        self.update_fd_interest(false);
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
