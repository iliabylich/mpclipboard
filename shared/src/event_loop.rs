#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::Timerfd;
use crate::Wants;
use alloc::rc::Rc;
use core::time::Duration;
use polling::{Event, Events, PollMode, Poller};
use rustix::{
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    io::Errno,
};

pub struct EventLoopWithTimer {
    poller: Rc<Poller>,
    events: Events,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    timer: Timerfd,
    #[cfg(target_os = "macos")]
    time: u64,
}

impl EventLoopWithTimer {
    const TIMER_KEY: usize = usize::MAX - 1;
    #[cfg(target_os = "macos")]
    const INITIAL_TIMER_KEY: usize = usize::MAX - 2;
    const FIRST_RESERVED_KEY: usize = usize::MAX - 2;

    pub fn new() -> Result<Self, EventLoopWithTimerError> {
        let poller = Poller::new().map_err(EventLoopWithTimerError::Create)?;

        #[cfg(any(target_os = "linux", target_os = "android"))]
        let this = {
            let timer = Timerfd::new().map_err(EventLoopWithTimerError::CreateTimer)?;
            unsafe {
                poller.add_with_mode(&timer, Event::readable(Self::TIMER_KEY), PollMode::Level)
            }
            .map_err(EventLoopWithTimerError::AddTimer)?;

            Self {
                poller: Rc::new(poller),
                events: Events::new(),
                timer,
            }
        };

        #[cfg(target_os = "macos")]
        let this = {
            use polling::os::kqueue::{PollerKqueueExt, Timer};

            let periodic = Timer {
                id: Self::TIMER_KEY,
                timeout: Duration::from_secs(1),
            };
            poller
                .add_filter(periodic, Self::TIMER_KEY, PollMode::Edge)
                .map_err(EventLoopWithTimerError::AddTimer)?;

            let initial = Timer {
                id: Self::INITIAL_TIMER_KEY,
                timeout: Duration::from_nanos(1),
            };
            poller
                .add_filter(initial, Self::INITIAL_TIMER_KEY, PollMode::Oneshot)
                .map_err(EventLoopWithTimerError::AddTimer)?;

            Self {
                poller: Rc::new(poller),
                events: Events::new(),
                time: 0,
            }
        };

        Ok(this)
    }

    pub fn register(
        &self,
        fd: OwnedFd,
        key: usize,
        wants: Wants,
    ) -> Result<RegisteredFd, EventLoopWithTimerError> {
        unsafe { self.add(fd.as_fd(), key, wants) }?;
        Ok(RegisteredFd {
            fd,
            key,
            poller: Rc::clone(&self.poller),
        })
    }

    /// # Safety
    ///
    /// `fd` must be [`delete()`](Self::delete)d before it is closed.
    pub unsafe fn add(
        &self,
        fd: BorrowedFd<'_>,
        key: usize,
        wants: Wants,
    ) -> Result<(), EventLoopWithTimerError> {
        let event = Self::event(key, wants)?;
        unsafe { self.poller.add_with_mode(&fd, event, PollMode::Level) }
            .map_err(EventLoopWithTimerError::Add)?;
        Ok(())
    }

    pub fn modify(
        &self,
        fd: BorrowedFd<'_>,
        key: usize,
        wants: Wants,
    ) -> Result<(), EventLoopWithTimerError> {
        let event = Self::event(key, wants)?;
        self.poller
            .modify_with_mode(fd, event, PollMode::Level)
            .map_err(EventLoopWithTimerError::Modify)?;
        Ok(())
    }

    pub fn delete(&self, fd: BorrowedFd<'_>) -> Result<(), EventLoopWithTimerError> {
        self.poller
            .delete(fd)
            .map_err(EventLoopWithTimerError::Delete)?;
        Ok(())
    }

    pub fn wait(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<EventLoopWithTimerResult<'_>, EventLoopWithTimerError> {
        self.events.clear();
        self.poller
            .wait(&mut self.events, timeout)
            .map_err(EventLoopWithTimerError::Wait)?;

        let mut time = None;

        for event in self.events.iter() {
            match event.key {
                #[cfg(any(target_os = "linux", target_os = "android"))]
                Self::TIMER_KEY => {
                    let now = self
                        .timer
                        .read()
                        .map_err(EventLoopWithTimerError::ReadTimer)?;
                    time = Some(now);
                }

                #[cfg(target_os = "macos")]
                Self::TIMER_KEY => {
                    self.time = self
                        .time
                        .checked_add(1)
                        .unwrap_or_else(|| unreachable!("seconds of uptime never overflow u64"));
                    time = Some(self.time);
                }

                #[cfg(target_os = "macos")]
                Self::INITIAL_TIMER_KEY => {
                    time = Some(self.time);
                }

                _ => {}
            }
        }

        Ok(EventLoopWithTimerResult {
            time,
            events: &self.events,
        })
    }

    const fn event(key: usize, wants: Wants) -> Result<Event, EventLoopWithTimerError> {
        if key >= Self::FIRST_RESERVED_KEY {
            return Err(EventLoopWithTimerError::ReservedKey(key));
        }

        let event = match wants {
            Wants::Read => Event::readable(key),
            Wants::Write => Event::writable(key),
            Wants::ReadWrite => Event::all(key),
        };
        Ok(event)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl Drop for EventLoopWithTimer {
    fn drop(&mut self) {
        let _ = self.poller.delete(self.timer.as_fd());
    }
}

impl AsFd for EventLoopWithTimer {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.poller.as_fd()
    }
}

impl AsRawFd for EventLoopWithTimer {
    fn as_raw_fd(&self) -> RawFd {
        self.poller.as_raw_fd()
    }
}

pub struct RegisteredFd {
    fd: OwnedFd,
    key: usize,
    poller: Rc<Poller>,
}

impl RegisteredFd {
    pub fn modify(&self, wants: Wants) -> Result<(), EventLoopWithTimerError> {
        let event = EventLoopWithTimer::event(self.key, wants)?;
        self.poller
            .modify_with_mode(&self.fd, event, PollMode::Level)
            .map_err(EventLoopWithTimerError::Modify)?;
        Ok(())
    }
}

impl Drop for RegisteredFd {
    fn drop(&mut self) {
        let _ = self.poller.delete(&self.fd);
    }
}

impl AsFd for RegisteredFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for RegisteredFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

pub struct EventLoopWithTimerResult<'a> {
    pub time: Option<u64>,
    events: &'a Events,
}

impl EventLoopWithTimerResult<'_> {
    pub fn fds(&self) -> impl Iterator<Item = (usize, EventLoopFdResult)> + '_ {
        self.events
            .iter()
            .filter(|event| event.key < EventLoopWithTimer::FIRST_RESERVED_KEY)
            .map(|event| {
                let fd = EventLoopFdResult {
                    readable: event.readable,
                    writable: event.writable,
                    has_error: event.is_err().unwrap_or(false) || event.is_interrupt(),
                };
                (event.key, fd)
            })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EventLoopFdResult {
    pub readable: bool,
    pub writable: bool,
    pub has_error: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum EventLoopWithTimerError {
    #[error("failed to create event loop: {0}")]
    Create(std::io::Error),
    #[error("failed to create timer: {0:?}")]
    CreateTimer(Errno),
    #[error("failed to register timer: {0}")]
    AddTimer(std::io::Error),
    #[error("failed to read timer: {0:?}")]
    ReadTimer(Errno),
    #[error("key {0} is reserved for timers")]
    ReservedKey(usize),
    #[error("failed to register fd: {0}")]
    Add(std::io::Error),
    #[error("failed to modify fd: {0}")]
    Modify(std::io::Error),
    #[error("failed to delete fd: {0}")]
    Delete(std::io::Error),
    #[error("failed to wait for events: {0}")]
    Wait(std::io::Error),
}
