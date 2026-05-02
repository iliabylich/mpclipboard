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
    /// Initializes `MPClipboard`, must be called once at the start of the program.
    /// Internally initializes logger and TLS.
    ///
    /// # Errors
    ///
    /// Returns an error if TLS initialization fails.
    pub fn init() -> Result<()> {
        Logger::init();
        TLS::init()?;
        Ok(())
    }

    /// Constructs a new instance
    pub fn new(context: Context) -> Self {
        let event_loop = Rc::clone(&context.event_loop);
        let state = State::start(context, Rc::clone(&event_loop));

        Self { event_loop, state }
    }

    /// Reads data from WebSocket connection, returns Output
    ///
    /// # Errors
    ///
    /// Returns an error if OS-specific event loop (epoll/kqueue) returns an error
    pub fn read(&mut self) -> Result<Option<Output>> {
        let event = self.event_loop.read()?;

        if event.timer {
            return self.state.tick();
        }

        if let Some((readable, writable)) = event.ws {
            return Ok(self.state.transition(readable, writable));
        }

        Ok(None)
    }

    /// Pushes a new text Clip with provided content.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    ///
    /// # Errors
    ///
    /// Returns an error if OS-specific event loop (epoll/kqueue) returns an error
    pub fn push_text(&mut self, text: String) -> Result<bool> {
        self.state.push(Clip::text(text))
    }
}

impl AsRawFd for MPClipboard {
    fn as_raw_fd(&self) -> i32 {
        self.event_loop.as_raw_fd()
    }
}

impl AsFd for MPClipboard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}
