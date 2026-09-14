use crate::{Output, config::Config, connection::Connection, logger::Logger, tls::TLS};
use anyhow::{Context, Result, bail};
use mpclipboard_shared::{EventLoop, EventLoopResult, Message, NonEmptyInlineString, Store};
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    sync::OnceLock,
};

pub struct MPClipboard {
    event_loop: EventLoop,
    now: u64,
    conn: Connection,
    store: Store,
}

impl MPClipboard {
    fn init_once() -> Result<()> {
        static INIT: OnceLock<Result<()>> = OnceLock::new();

        let result = INIT.get_or_init(|| {
            Logger::init();
            TLS::init()
        });

        match result {
            Ok(()) => Ok(()),
            Err(err) => bail!("failed to init_once() MPClipboard: {err:?}"),
        }
    }

    fn new(config: Config) -> Result<Self> {
        log::info!("Running with config {config:?}");
        let mut event_loop = EventLoop::new().context("event loop has crashed")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| unreachable!("time goes backwards"))
            .as_secs();
        let conn = Connection::new(config);

        event_loop
            .sync(conn.wants())
            .context("failed to update connection fd in event loop")?;

        Ok(Self {
            event_loop,
            now,
            conn,
            store: Store::empty(),
        })
    }

    pub fn new_inline(url: &str, token: &str, id: &str) -> Result<Self> {
        Self::init_once()?;
        let config = Config::new(url, token, id)?;
        Self::new(config)
    }

    pub fn new_with_local_config() -> Result<Self> {
        Self::init_once()?;
        let config = Config::read_local_file()?;
        Self::new(config)
    }

    pub fn new_with_local_config_and_id_override(id: &str) -> Result<Self> {
        Self::init_once()?;
        let mut config = Config::read_local_file()?;
        config.id = NonEmptyInlineString::new(id).context("malformed id override")?;
        Self::new(config)
    }

    pub fn new_with_xdg_config() -> Result<Self> {
        Self::init_once()?;
        let config = Config::read_in_xdg_config_dir()?;
        Self::new(config)
    }

    pub fn read(&mut self) -> Result<Option<Output>> {
        let polled = self
            .event_loop
            .drain_events_without_waiting()
            .context("failed to drain event loop")?;

        let prev_connectivity = self.conn.connectivity();
        let message = if let Some(message) = self.drain(&polled)
            && self.store.add(message)
        {
            Some(message.text_as_str().to_string())
        } else {
            None
        };
        let next_connectivity = self.conn.connectivity();

        self.event_loop
            .sync(self.conn.wants())
            .context("failed to update connection fd in event loop")?;

        let connectivity = if prev_connectivity == next_connectivity {
            None
        } else {
            Some(next_connectivity)
        };

        Ok(match (connectivity, message) {
            (Some(connectivity), Some(text)) => Some(Output::Both { connectivity, text }),
            (Some(connectivity), None) => Some(Output::ConnectivityChanged { connectivity }),
            (None, Some(text)) => Some(Output::NewText { text }),
            (None, None) => None,
        })
    }

    fn drain(&mut self, polled: &EventLoopResult) -> Option<Message> {
        let mut out = None;

        if let Some(time) = polled.time {
            self.now = time;
            log::trace!("tick {}", self.now);
            self.conn.tick(self.now);
        }

        if let Some((readable, writable, has_error)) = polled.fd {
            if has_error && !self.conn.is_disconnected() {
                log::error!("poll() returned connection error, disconnecting");
                self.conn.force_disconnect(self.now);
            }

            if readable && !self.conn.is_disconnected() {
                out = self.conn.on_readable(self.now);
            }

            if writable && !self.conn.is_disconnected() {
                self.conn.on_writable(self.now);
            }
        }

        out
    }

    pub fn push_text(&mut self, text: &str) -> Result<bool> {
        if text.is_empty() {
            log::info!("Skipping empty text");
            return Ok(false);
        }

        let text = NonEmptyInlineString::truncate(text)?;
        let message = Message::new(text);

        if !self.store.add(message) {
            return Ok(false);
        }

        let pushed = self.conn.push(message);

        self.event_loop
            .sync(self.conn.wants())
            .context("failed to update connection fd in event loop")?;

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
