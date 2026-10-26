mod result;
pub(crate) use result::EventLoopResult;

use core::time::Duration;
use mpclipboard_shared::{
    EventLoopAwareFd, EventLoopWithTimer, EventLoopWithTimerError, Now, Wants,
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::time::Instant;

pub(crate) struct EventLoop {
    inner: EventLoopWithTimer,
}

impl EventLoop {
    const FD_KEY: usize = 0;

    pub(crate) fn new() -> Result<Self, EventLoopWithTimerError> {
        let inner = EventLoopWithTimer::new()?;
        Ok(Self { inner })
    }

    pub(crate) fn register(
        &self,
        fd: OwnedFd,
        wants: Wants,
    ) -> Result<EventLoopAwareFd, EventLoopWithTimerError> {
        self.inner.register(fd, Self::FD_KEY, wants)
    }

    pub(crate) fn set_deadline(
        &mut self,
        deadline: Option<Instant>,
    ) -> Result<(), EventLoopWithTimerError> {
        self.inner.set_deadline(deadline)
    }

    pub(crate) fn drain_events_without_waiting(
        &mut self,
    ) -> Result<EventLoopResult, EventLoopWithTimerError> {
        let polled = self.inner.wait(Some(Duration::ZERO))?;

        let mut out = EventLoopResult {
            ticked: polled.ticked,
            fd: None,
        };

        for (_, revents) in polled.fds() {
            let fd = out.fd.get_or_insert_default();
            fd.readable |= revents.readable;
            fd.writable |= revents.writable;
            fd.has_error |= revents.has_error;
        }

        Ok(out)
    }

    pub(crate) const fn now(&self) -> Now {
        self.inner.now()
    }
}

impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.as_fd()
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}
