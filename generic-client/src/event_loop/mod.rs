use anyhow::{Context as _, Result, bail};
use polling::{Event, Events, PollMode, Poller};
use std::{
    cell::Cell,
    os::fd::{AsRawFd, BorrowedFd},
    rc::Rc,
    time::Duration,
};

mod mid_handshake;
mod owned_fd;
mod websocket;

pub(crate) use mid_handshake::EventLoopAwareMidHandshake;
pub(crate) use owned_fd::EventLoopAwareOwnedFd;
pub(crate) use websocket::{EventLoopAwareWebSocket, ReadResult, WriteResult};

pub(crate) struct EventLoop {
    poller: Poller,
    ticks: Cell<u64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EventLoopEvent {
    pub(crate) tick: Option<u64>,
    pub(crate) ws: Option<(bool, bool)>,
}

impl EventLoop {
    const TIMER_ID: u32 = 1;
    const WS_ID: u32 = 2;

    pub(crate) fn new() -> Result<Rc<Self>> {
        let this = Self {
            poller: Poller::new()?,
            ticks: Cell::new(0),
        };
        log::trace!("Supports edge: {}", this.poller.supports_edge());
        log::trace!("Supports level: {}", this.poller.supports_level());
        this.add_timer()?;
        Ok(Rc::new(this))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn add_timer(&self) -> Result<()> {
        use polling::os::kqueue::{PollerKqueueExt, Timer};
        self.poller.add_filter(
            Timer {
                id: 2,
                timeout: Duration::from_secs(1),
            },
            Self::TIMER_ID as usize,
            PollMode::Level,
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn drain_timer(&self) -> Result<()> {
        Ok(())
    }

    pub(crate) fn add(&self, fd: i32, readable: bool, writable: bool) -> Result<()> {
        unsafe {
            self.poller
                .add_with_mode(
                    fd,
                    Event::new(Self::WS_ID as usize, readable, writable),
                    PollMode::Level,
                )
                .context("failed to add FD to Poller")
        }
    }

    pub(crate) fn remove(&self, fd: i32) {
        if let Err(err) = self.poller.delete(unsafe { BorrowedFd::borrow_raw(fd) }) {
            log::error!("failed to delete FD from Poller: {err:?}")
        }
    }

    pub(crate) fn modify(&self, fd: i32, readable: bool, writable: bool) -> Result<()> {
        self.poller
            .modify_with_mode(
                unsafe { BorrowedFd::borrow_raw(fd) },
                Event::new(Self::WS_ID as usize, readable, writable),
                PollMode::Level,
            )
            .context("failed to modify FD in Poller")
    }

    pub(crate) fn read(&self) -> Result<EventLoopEvent> {
        let mut events = Events::new();

        self.poller
            .wait(&mut events, None)
            .context("failed to read")?;

        let mut tick = None;
        let mut ws = None;

        for event in events.iter() {
            match event.key as u32 {
                Self::TIMER_ID => {
                    self.ticks.set(self.ticks.get() + 1);
                    self.drain_timer()?;
                    tick = Some(self.ticks.get());
                }

                Self::WS_ID => ws = Some((event.readable, event.writable)),

                _ => {
                    bail!("unknown event: {event:?}")
                }
            }
        }

        Ok(EventLoopEvent { tick, ws })
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.poller.as_raw_fd()
    }
}
