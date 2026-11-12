use crate::{
    Output,
    config::{Config, ConfigError},
    logger::Logger,
    runtime::{Runtime, RuntimeError},
    tls_config::tls_config,
};
use mpclipboard_shared::{Message, Store};
use rustix::fd::{AsFd, BorrowedFd};
use std::sync::Once;

pub struct MPClipboard {
    runtime: Runtime,
    store: Store,
}

impl MPClipboard {
    fn new(config: Config) -> Result<Self, MPClipboardError> {
        log::info!("Running with config {config:?}");
        let tls_config = tls_config()?;
        let runtime = Runtime::new(config, tls_config)?;

        Ok(Self {
            runtime,
            store: Store::default(),
        })
    }

    pub fn new_inline(url: &str, token: &str, id: &str) -> Result<Self, MPClipboardError> {
        init_once();
        let config = Config::new(url, token, id)?;
        Self::new(config)
    }

    pub fn new_from_file() -> Result<Self, MPClipboardError> {
        init_once();
        let config = Config::new_from_file()?;
        Self::new(config)
    }

    pub fn read(&mut self) -> Result<Output, MPClipboardError> {
        self.runtime.drain()?;

        let (connectivity, received) = self.runtime.state_mut().take_updates();

        let text = match received {
            Some(message) if self.store.add(message) => Some(message.text().to_string()),
            _ => None,
        };

        Ok(Output { connectivity, text })
    }

    pub fn push_text(&mut self, text: &str) -> Result<bool, MPClipboardError> {
        let Some(message) = Message::truncate(text) else {
            log::info!("Skipping empty text");
            return Ok(false);
        };

        if !self.store.add(message) {
            return Ok(false);
        }

        self.runtime.push(message)?;
        Ok(true)
    }
}

impl AsFd for MPClipboard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.runtime.as_fd()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MPClipboardError {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("TLS config error: {0}")]
    TlsConfig(#[from] rustls::Error),
    #[error("runtime error: {0}")]
    Runtime(#[from] RuntimeError),
}

fn init_once() {
    static INIT: Once = Once::new();

    INIT.call_once(Logger::init);
}
