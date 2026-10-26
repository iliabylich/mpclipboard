#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::timerfd::Timerfd;
use crate::{
    Now, Wants,
    polling::{Event, Events, PollMode, Poller},
};
use alloc::rc::Rc;
use core::time::Duration;
use rustix::{
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    io::Errno,
};
use std::time::Instant;

pub struct EventLoopWithTimer {
    poller: Rc<Poller>,
    events: Events,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    timer: Timerfd,
    start: Instant,
    seconds: u64,
    next_tick: Instant,
    deadline: Option<Instant>,
    armed: Instant,
}

impl EventLoopWithTimer {
    const TIMER_KEY: usize = usize::MAX - 1;
    const FIRST_RESERVED_KEY: usize = usize::MAX - 1;

    pub fn new() -> Result<Self, EventLoopWithTimerError> {
        let poller = Poller::new().map_err(EventLoopWithTimerError::Create)?;
        let start = Instant::now();

        #[cfg(any(target_os = "linux", target_os = "android"))]
        let timer = {
            let timer = Timerfd::new().map_err(EventLoopWithTimerError::CreateTimer)?;
            timer
                .set(Duration::ZERO)
                .map_err(EventLoopWithTimerError::SetTimer)?;
            unsafe {
                poller.add_with_mode(&timer, Event::readable(Self::TIMER_KEY), PollMode::Level)
            }
            .map_err(EventLoopWithTimerError::AddTimer)?;
            timer
        };

        #[cfg(target_os = "macos")]
        {
            use crate::polling::os::kqueue::PollerKqueueExt;

            poller
                .add_filter(
                    Self::kqueue_timer(Duration::ZERO),
                    Self::TIMER_KEY,
                    PollMode::Oneshot,
                )
                .map_err(EventLoopWithTimerError::AddTimer)?;
        }

        Ok(Self {
            poller: Rc::new(poller),
            events: Events::new(),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            timer,
            start,
            seconds: 0,
            next_tick: start,
            deadline: None,
            armed: start,
        })
    }

    pub fn register(
        &self,
        fd: OwnedFd,
        key: usize,
        wants: Wants,
    ) -> Result<RegisteredFd, EventLoopWithTimerError> {
        let event = Self::event(key, wants)?;
        unsafe { self.poller.add_with_mode(&fd, event, PollMode::Level) }
            .map_err(EventLoopWithTimerError::Add)?;
        Ok(RegisteredFd {
            fd,
            key,
            poller: Rc::clone(&self.poller),
        })
    }

    pub fn set_deadline(
        &mut self,
        deadline: Option<Instant>,
    ) -> Result<(), EventLoopWithTimerError> {
        if self.deadline == deadline {
            return Ok(());
        }

        self.deadline = deadline;
        let target = self.target();
        if target != self.armed {
            self.arm(target)?;
        }
        Ok(())
    }

    fn target(&self) -> Instant {
        self.deadline
            .map_or(self.next_tick, |deadline| deadline.min(self.next_tick))
    }

    fn arm(&mut self, target: Instant) -> Result<(), EventLoopWithTimerError> {
        let timeout = target.saturating_duration_since(Instant::now());

        #[cfg(any(target_os = "linux", target_os = "android"))]
        self.timer
            .set(timeout)
            .map_err(EventLoopWithTimerError::SetTimer)?;

        #[cfg(target_os = "macos")]
        {
            use crate::polling::os::kqueue::PollerKqueueExt;

            self.poller
                .modify_filter(
                    Self::kqueue_timer(timeout),
                    Self::TIMER_KEY,
                    PollMode::Oneshot,
                )
                .map_err(EventLoopWithTimerError::SetTimer)?;
        }

        self.armed = target;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn kqueue_timer(timeout: Duration) -> crate::polling::os::kqueue::Timer {
        crate::polling::os::kqueue::Timer {
            id: Self::TIMER_KEY,
            timeout: timeout.max(Duration::from_nanos(1)),
        }
    }

    pub const fn now(&self) -> Now {
        Now::new(self.seconds)
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn wait(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<EventLoopWithTimerResult<'_>, EventLoopWithTimerError> {
        self.events.clear();
        self.poller
            .wait(&mut self.events, timeout)
            .map_err(EventLoopWithTimerError::Wait)?;

        let mut ticked = false;
        let mut deadline_expired = false;

        if self.events.iter().any(|event| event.key == Self::TIMER_KEY) {
            let now = Instant::now();

            if now >= self.next_tick {
                ticked = true;
                self.seconds = now.duration_since(self.start).as_secs();
                self.next_tick = self
                    .seconds
                    .checked_add(1)
                    .and_then(|seconds| self.start.checked_add(Duration::from_secs(seconds)))
                    .unwrap_or_else(|| unreachable!("seconds of uptime never overflow"));
            }

            if self.deadline.is_some_and(|deadline| now >= deadline) {
                self.deadline = None;
                deadline_expired = true;
            }

            assert!(
                ticked || deadline_expired,
                "timer woke up with nothing due: now={now:?}, armed={:?}, next_tick={:?}, deadline={:?}",
                self.armed,
                self.next_tick,
                self.deadline,
            );

            self.arm(self.target())?;
        }

        Ok(EventLoopWithTimerResult {
            ticked,
            #[cfg(test)]
            deadline_expired,
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

#[derive(Debug)]
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
    pub ticked: bool,
    #[cfg(test)]
    pub deadline_expired: bool,
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
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("failed to set timer: {0:?}")]
    SetTimer(Errno),
    #[cfg(target_os = "macos")]
    #[error("failed to set timer: {0}")]
    SetTimer(std::io::Error),
    #[error("key {0} is reserved for timers")]
    ReservedKey(usize),
    #[error("failed to register fd: {0}")]
    Add(std::io::Error),
    #[error("failed to modify fd: {0}")]
    Modify(std::io::Error),
    #[error("failed to wait for events: {0}")]
    Wait(std::io::Error),
}
