use crate::timer::Timer;
use anyhow::{Context, Result, bail};
use polling::{Event, Events, PollMode, Poller};
use std::{
    os::fd::{AsRawFd, BorrowedFd, OwnedFd},
    rc::Rc,
};

pub(crate) struct EventLoop {
    poller: Poller,
    timer: Timer,
    #[allow(dead_code)]
    timerfd: Option<OwnedFd>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EventLoopEvent {
    pub(crate) timer: bool,
    pub(crate) ws: Option<(bool, bool)>,
}

impl EventLoop {
    const TIMER_ID: usize = 1;
    const WS_ID: usize = 2;

    pub(crate) fn new() -> Result<Rc<Self>> {
        let mut this = Self {
            poller: Poller::new().context("bug: failed to instantiate event loop")?,
            timer: Timer::new(),
            timerfd: None,
        };
        log::trace!("Supports edge: {}", this.poller.supports_edge());
        log::trace!("Supports level: {}", this.poller.supports_level());
        this.add_timer()?;
        Ok(Rc::new(this))
    }

    pub(crate) fn timer(&self) -> Timer {
        self.timer.clone()
    }

    pub(crate) fn add(&self, fd: i32, readable: bool, writable: bool) -> Result<()> {
        unsafe {
            self.poller
                .add_with_mode(
                    fd,
                    Event::new(Self::WS_ID, readable, writable),
                    PollMode::Level,
                )
                .context("bug: failed to add FD to Poller")?;
        }
        Ok(())
    }

    pub(crate) fn remove(&self, fd: i32) {
        if let Err(err) = self.poller.delete(unsafe { BorrowedFd::borrow_raw(fd) }) {
            log::error!("failed to delete FD from Poller: {err:?}");
        }
    }

    pub(crate) fn modify(&self, fd: i32, readable: bool, writable: bool) -> Result<()> {
        self.poller
            .modify_with_mode(
                unsafe { BorrowedFd::borrow_raw(fd) },
                Event::new(Self::WS_ID, readable, writable),
                PollMode::Level,
            )
            .context("bug: failed to modify FD in Poller")?;
        Ok(())
    }

    pub(crate) fn read(&self) -> Result<EventLoopEvent> {
        let mut events = Events::new();

        self.poller
            .wait(&mut events, None)
            .context("bug: failed to read")?;

        let mut out = EventLoopEvent {
            timer: false,
            ws: None,
        };

        for event in events.iter() {
            match event.key {
                Self::TIMER_ID => {
                    self.timer.tick();
                    self.drain_timer()?;
                    out.timer = true;
                }

                Self::WS_ID => out.ws = Some((event.readable, event.writable)),

                _ => {
                    bail!("bug: unknown event: {event:?}")
                }
            }
        }

        Ok(out)
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> i32 {
        self.poller.as_raw_fd()
    }
}

trait AddTimer {
    fn add_timer(&mut self) -> Result<()>;
    fn drain_timer(&self) -> Result<()>;
}

#[cfg(target_os = "macos")]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> Result<()> {
        use polling::os::kqueue::{PollerKqueueExt, Timer};
        self.poller
            .add_filter(
                Timer {
                    id: 2,
                    timeout: std::time::Duration::from_secs(1),
                },
                Self::TIMER_ID,
                PollMode::Level,
            )
            .context("bug: failed to add timer")?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn drain_timer(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> Result<()> {
        use rustix::time::{
            Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
            timerfd_settime,
        };

        let timerfd = timerfd_create(TimerfdClockId::Monotonic, TimerfdFlags::NONBLOCK)
            .context("bug: failed to create timerfd")?;

        timerfd_settime(
            &timerfd,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
            },
        )
        .context("bug: failed to configure timer")?;

        unsafe {
            self.poller
                .add_with_mode(
                    &timerfd,
                    Event::new(Self::TIMER_ID, true, false),
                    PollMode::Level,
                )
                .context("bug: failed to add timer to epoll")?;
        }

        self.timerfd = Some(timerfd);
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn drain_timer(&self) -> Result<()> {
        let mut buf = [0_u8; 8];
        let bytes_read = rustix::io::read(
            self.timerfd.as_ref().context("bug: timerfd isn't set")?,
            &mut buf,
        )
        .context("bug: failed to read from timer")?;
        assert_eq!(bytes_read, 8);
        Ok(())
    }
}
