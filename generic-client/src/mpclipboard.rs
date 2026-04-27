use crate::{Context, Output, event_loop::EventLoop, logger::Logger, state::State, tls::TLS};
use anyhow::Result;
use clip::Clip;
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    rc::Rc,
};

/// The main entrypoint
pub struct MPClipboard {
    event_loop: Rc<EventLoop>,
    state: State,
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
        let event_loop = Rc::clone(&context.event_loop);
        let state = State::start(context);

        Self { event_loop, state }
    }

    /// Reads data from WebSocket connection, returns Output
    pub fn read(&mut self) -> Option<Output> {
        let event = self.event_loop.read();

        if event.timer {
            return self.state.tick();
        }

        if let Some((readable, writable)) = event.ws {
            return self.state.transition(readable, writable);
        }

        None
    }

    /// Pushes a new text Clip with provided content.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    #[must_use]
    pub fn push_text(&mut self, text: String) -> bool {
        self.state.push(Clip::text(text))
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
