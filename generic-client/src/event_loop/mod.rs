mod epoch;
pub(crate) use epoch::Epoch;

mod fd_state;
use fd_state::{Diff, FdState};

mod result;
pub(crate) use result::EventLoopResult;

use core::time::Duration;
use mpclipboard_shared::{EventLoopWithTimer, EventLoopWithTimerError, Wants};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

pub(crate) struct EventLoop {
    inner: EventLoopWithTimer,
    fd: FdState,
    registered: Option<RawFd>,
}

impl EventLoop {
    const FD_KEY: usize = 0;

    pub(crate) fn new() -> Result<Self, EventLoopWithTimerError> {
        let inner = EventLoopWithTimer::new()?;

        Ok(Self {
            inner,
            fd: FdState::None,
            registered: None,
        })
    }

    pub(crate) fn sync(
        &mut self,
        wants: Option<(BorrowedFd<'_>, Epoch, Wants)>,
        deadline: Option<Duration>,
    ) -> Result<(), EventLoopWithTimerError> {
        self.inner.set_deadline(deadline)?;

        match self.fd.transition(wants) {
            Diff::Add { fd, wants } => {
                self.forget_registered();
                unsafe { self.inner.add(fd, Self::FD_KEY, wants) }?;
                self.registered = Some(fd.as_raw_fd());
            }
            Diff::Modify { fd, wants } => {
                self.inner.modify(fd, Self::FD_KEY, wants)?;
            }
            Diff::Empty => {}
        }

        if matches!(self.fd, FdState::None) {
            self.forget_registered();
        }

        Ok(())
    }

    pub(crate) fn drain_events_without_waiting(
        &mut self,
    ) -> Result<EventLoopResult, EventLoopWithTimerError> {
        let polled = self.inner.wait(Some(Duration::ZERO))?;

        let mut out = EventLoopResult {
            time: polled.time,
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

    fn forget_registered(&mut self) {
        let Some(fd) = self.registered.take() else {
            return;
        };
        let _ = self.inner.delete(unsafe { BorrowedFd::borrow_raw(fd) });
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
