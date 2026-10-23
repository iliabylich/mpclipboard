use crate::{
    Output,
    config::{Config, ConfigError},
    connection::{Connection, QuicConfigError},
    event_loop::{EventLoop, EventLoopResult},
    logger::Logger,
};
use mpclipboard_shared::{EventLoopWithTimerError, Message, Store};
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    sync::Once,
};

pub struct MPClipboard {
    event_loop: EventLoop,
    conn: Connection,
    store: Store,
    config: Config,
}

impl MPClipboard {
    fn init_once() {
        static INIT: Once = Once::new();

        INIT.call_once(Logger::init);
    }

    fn new(config: Config) -> Result<Self, MPClipboardError> {
        log::info!("Running with config {config:?}");
        let event_loop = EventLoop::new()?;
        let conn = Connection::new()?;

        let mut this = Self {
            event_loop,
            conn,
            store: Store::default(),
            config,
        };
        this.sync_event_loop()?;
        Ok(this)
    }

    fn sync_event_loop(&mut self) -> Result<(), MPClipboardError> {
        self.conn.flush(self.event_loop.now());
        let deadline = self.conn.next_timer();
        let wants = self.conn.wants();
        self.event_loop.sync(wants, deadline)?;
        Ok(())
    }

    pub fn new_inline(url: &str, token: &str, id: &str) -> Result<Self, MPClipboardError> {
        Self::init_once();
        let config = Config::new(url, token, id)?;
        Self::new(config)
    }

    pub fn new_from_file() -> Result<Self, MPClipboardError> {
        Self::init_once();
        let config = Config::new_from_file()?;
        Self::new(config)
    }

    pub fn read(&mut self) -> Result<Output, MPClipboardError> {
        let polled = self.event_loop.drain_events_without_waiting()?;

        let prev_connectivity = self.conn.connectivity();
        let text = if let Some(message) = self.drain(&polled)
            && self.store.add(&message)
        {
            Some(message.text_as_str().to_string())
        } else {
            None
        };
        let next_connectivity = self.conn.connectivity();

        log::trace!("Connection wants: {:?}", self.conn.wants());
        self.sync_event_loop()?;

        let connectivity = if prev_connectivity == next_connectivity {
            None
        } else {
            Some(next_connectivity)
        };

        Ok(Output { connectivity, text })
    }

    fn drain(&mut self, polled: &EventLoopResult) -> Option<Message> {
        let now = self.event_loop.now();
        let mut out = None;

        if polled.ticked {
            log::trace!("tick {now}");
            self.conn.tick(now, &self.config);
        }

        if let Some(fd) = polled.fd {
            if fd.has_error && !self.conn.is_disconnected() {
                log::error!("poll() returned connection error, disconnecting");
                self.conn.disconnect(now);
            }

            if fd.readable && !self.conn.is_disconnected() {
                out = self.conn.on_readable(now, &self.config);
            }

            if fd.writable && !self.conn.is_disconnected() {
                self.conn.on_writable(now);
            }
        }

        self.conn.process_expired_timers(now);

        out
    }

    pub fn push_text(&mut self, text: &str) -> Result<bool, MPClipboardError> {
        if text.is_empty() {
            log::info!("Skipping empty text");
            return Ok(false);
        }

        let message = Message::truncate(text)
            .unwrap_or_else(|_| unreachable!("non-empty text truncated to MAXLEN is always valid"));

        if !self.store.add(&message) {
            return Ok(false);
        }

        let pushed = self.conn.push(&message, self.event_loop.now());
        self.sync_event_loop()?;

        Ok(pushed)
    }
}

impl AsRawFd for MPClipboard {
    fn as_raw_fd(&self) -> i32 {
        self.event_loop.as_raw_fd()
    }
}

impl AsFd for MPClipboard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.event_loop.as_fd()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MPClipboardError {
    #[error("config error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("event loop error: {0}")]
    EventLoopError(#[from] EventLoopWithTimerError),
    #[error("QUIC config error: {0}")]
    QuicConfigError(#[from] QuicConfigError),
}
