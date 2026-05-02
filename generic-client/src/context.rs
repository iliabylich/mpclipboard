use crate::{Config, event_loop::EventLoop, timer::Timer, tls::TLS};
use anyhow::Result;
use clip::Clip;
use std::{net::SocketAddr, rc::Rc};
use tungstenite::Message;

/// Execution context of `MPClipboard`, once constructed nothing can fail
pub struct Context {
    pub(crate) config: Config,
    pub(crate) remote_addr: SocketAddr,
    pub(crate) tls: TLS,
    pub(crate) event_loop: Rc<EventLoop>,
    pub(crate) timer: Timer,
    pub(crate) pending_message_to_send: Option<Message>,
    pub(crate) last_clip: Clip,
    pub(crate) last_time_worked_with_ws_at: u64,
}

impl Context {
    /// Constructs a new context
    /// Internally builds TLS connector and epoll/kqueue event loop.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// 1. TLS can't be configured
    /// 2. DNS name of the remote server can't be resolved
    /// 3. OS-specific event loop (epoll/kqueue) can't be initialized
    pub fn new(config: Config) -> Result<Self> {
        let enable_tls = config.enable_tls()?;
        let tls = TLS::new(enable_tls)?;

        let remote_addr = config.remote_addr()?;
        log::trace!("remote_addr = {remote_addr:?}");

        let event_loop = EventLoop::new()?;
        let timer = event_loop.timer();

        Ok(Self {
            config,
            remote_addr,
            tls,
            event_loop,
            timer,
            pending_message_to_send: None,
            last_clip: Clip::zero(),
            last_time_worked_with_ws_at: 0,
        })
    }
}
