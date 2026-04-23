use crate::{Config, event_loop::EventLoop, tls::TLS};
use anyhow::Result;
use std::{net::SocketAddr, rc::Rc};

/// Execution context of MPClipboard, once constructed nothing can fail
pub struct Context {
    pub(crate) config: Config,
    pub(crate) remote_addr: SocketAddr,
    pub(crate) tls: TLS,
    pub(crate) event_loop: Rc<EventLoop>,
}

impl Context {
    /// Constructs a new context
    /// Internally builds TLS connector and epoll/kqueue event loop.
    pub fn new(config: Config) -> Result<Self> {
        let enable_tls = config.enable_tls()?;
        let tls = TLS::new(enable_tls)?;

        let remote_addr = config.remote_addr()?;
        log::trace!("remote_addr = {remote_addr:?}");

        let event_loop = EventLoop::new();

        Ok(Self {
            config,
            remote_addr,
            tls,
            event_loop,
        })
    }
}
