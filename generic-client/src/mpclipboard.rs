use crate::{
    Connectivity, Output,
    config::{Config, ConfigError},
    connection::Connection,
    logger::Logger,
    tls::TLS,
};
use mpclipboard_shared::{
    Epoch, EventLoop, EventLoopError, EventLoopResult, Message, NonEmptyInlineString, Store,
};
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    sync::Once,
};

pub struct MPClipboard {
    event_loop: EventLoop,
    now: u64,
    conn: Connection,
    epoch: Epoch,
    store: Store,
    config: Config,
}

impl MPClipboard {
    fn init_once() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            Logger::init();
            TLS::init();
        });
    }

    fn new(config: Config) -> Result<Self, MPClipboardError> {
        log::info!("Running with config {config:?}");
        let event_loop = EventLoop::new().map_err(MPClipboardError::EventLoopError)?;

        let mut this = Self {
            event_loop,
            now: 0,
            conn: Connection::new(),
            epoch: Epoch::new(),
            store: Store::empty(),
            config,
        };
        this.sync_event_loop()?;
        Ok(this)
    }

    fn sync_event_loop(&mut self) -> Result<(), MPClipboardError> {
        let wants = self.conn.wants().map(|(fd, wants)| (fd, self.epoch, wants));
        self.event_loop
            .sync(wants)
            .map_err(MPClipboardError::EventLoopError)
    }

    pub fn new_inline(url: &str, token: &str, id: &str) -> Result<Self, MPClipboardError> {
        Self::init_once();
        let config = Config::new(url, token, id).map_err(MPClipboardError::ConfigError)?;
        Self::new(config)
    }

    pub fn new_with_local_config() -> Result<Self, MPClipboardError> {
        Self::init_once();
        let config = Config::read_local_file().map_err(MPClipboardError::ConfigError)?;
        Self::new(config)
    }

    pub fn new_with_local_config_and_id_override(id: &str) -> Result<Self, MPClipboardError> {
        Self::init_once();
        let mut config = Config::read_local_file().map_err(MPClipboardError::ConfigError)?;
        config.id = NonEmptyInlineString::new(id)
            .map_err(ConfigError::Id)
            .map_err(MPClipboardError::ConfigError)?;
        Self::new(config)
    }

    pub fn new_with_xdg_config() -> Result<Self, MPClipboardError> {
        Self::init_once();
        let config = Config::read_in_xdg_config_dir().map_err(MPClipboardError::ConfigError)?;
        Self::new(config)
    }

    pub fn read(&mut self) -> Result<Output, MPClipboardError> {
        let polled = self
            .event_loop
            .drain_events_without_waiting()
            .map_err(MPClipboardError::EventLoopError)?;

        let prev_connectivity = Connectivity::new(&self.conn);
        let text = if let Some(message) = self.drain(&polled)
            && self.store.add(message)
        {
            Some(message.text_as_str().to_string())
        } else {
            None
        };
        let next_connectivity = Connectivity::new(&self.conn);

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
        let mut out = None;

        if let Some(time) = polled.time {
            self.now = time;
            log::trace!("tick {}", self.now);
            let was_disconnected = self.conn.is_disconnected();
            self.conn.tick(self.now, &self.config);
            if was_disconnected && !self.conn.is_disconnected() {
                self.epoch.bump();
            }
        }

        if let Some((readable, writable, has_error)) = polled.fd {
            if has_error && !self.conn.is_disconnected() {
                log::error!("poll() returned connection error, disconnecting");
                self.conn.force_disconnect(self.now);
            }

            if readable && !self.conn.is_disconnected() {
                out = self.conn.on_readable(self.now, &self.config);
            }

            if writable && !self.conn.is_disconnected() {
                self.conn.on_writable(self.now, &self.config);
            }
        }

        out
    }

    pub fn push_text(&mut self, text: &str) -> Result<bool, MPClipboardError> {
        if text.is_empty() {
            log::info!("Skipping empty text");
            return Ok(false);
        }

        let text = NonEmptyInlineString::truncate(text)
            .unwrap_or_else(|_| unreachable!("non-empty text truncated to MAXLEN is always valid"));
        let message = Message::new(text);

        if !self.store.add(message) {
            return Ok(false);
        }

        let pushed = self.conn.push(message);
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
    ConfigError(ConfigError),
    #[error("event loop error: {0}")]
    EventLoopError(EventLoopError),
}
