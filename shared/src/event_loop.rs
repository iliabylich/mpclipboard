use crate::{
    EventLoopAwareFd, Now, Wants,
    polling::{Event, Events, PollMode, Poller},
    timer::Timer,
};
use alloc::rc::Rc;
use core::time::Duration;
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    io::Errno,
};
use std::time::Instant;

pub struct EventLoopWithTimer {
    poller: Rc<Poller>,
    events: Events,
    timer: Timer,
    start: Instant,
    seconds: u64,
    next_tick: Instant,
    deadline: Option<Instant>,
}

impl EventLoopWithTimer {
    const TIMER_KEY: usize = usize::MAX - 1;

    pub fn new() -> Result<Self, EventLoopWithTimerError> {
        let poller = Rc::new(Poller::new().map_err(EventLoopWithTimerError::Create)?);
        let start = Instant::now();
        let timer = Timer::new(Rc::clone(&poller), Self::TIMER_KEY)?;

        Ok(Self {
            poller,
            events: Events::new(),
            timer,
            start,
            seconds: 0,
            next_tick: start,
            deadline: None,
        })
    }

    pub fn register(
        &self,
        fd: OwnedFd,
        key: usize,
        wants: Wants,
    ) -> Result<EventLoopAwareFd, EventLoopWithTimerError> {
        let event = Self::event(key, wants)?;
        unsafe { self.poller.add_with_mode(&fd, event, PollMode::Level) }
            .map_err(EventLoopWithTimerError::Add)?;
        Ok(EventLoopAwareFd::new(fd, key, Rc::clone(&self.poller)))
    }

    pub fn set_deadline(
        &mut self,
        deadline: Option<Instant>,
    ) -> Result<(), EventLoopWithTimerError> {
        let prev = self.target();
        self.deadline = deadline;
        if self.target() != prev {
            self.arm()?;
        }
        Ok(())
    }

    fn target(&self) -> Instant {
        self.deadline
            .map_or(self.next_tick, |deadline| deadline.min(self.next_tick))
    }

    fn arm(&self) -> Result<(), EventLoopWithTimerError> {
        let timeout = self.target().saturating_duration_since(Instant::now());
        self.timer.set(timeout)?;
        Ok(())
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
                "timer woke up with nothing due: now={now:?}, next_tick={:?}, deadline={:?}",
                self.next_tick,
                self.deadline,
            );

            self.arm()?;
        }

        Ok(EventLoopWithTimerResult {
            ticked,
            #[cfg(test)]
            deadline_expired,
            events: &self.events,
        })
    }

    pub(crate) const fn event(key: usize, wants: Wants) -> Result<Event, EventLoopWithTimerError> {
        if key == Self::TIMER_KEY {
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

impl AsFd for EventLoopWithTimer {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.poller.as_fd()
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
            .filter(|event| event.key != EventLoopWithTimer::TIMER_KEY)
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
