use crate::{
    EventLoopWithTimer, EventLoopWithTimerError, Wants,
    polling::{PollMode, Poller},
};
use alloc::rc::Rc;
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};

#[derive(Debug)]
pub struct EventLoopAwareFd {
    fd: OwnedFd,
    key: usize,
    poller: Rc<Poller>,
}

impl EventLoopAwareFd {
    pub(crate) const fn new(fd: OwnedFd, key: usize, poller: Rc<Poller>) -> Self {
        Self { fd, key, poller }
    }

    pub fn modify(&self, wants: Wants) -> Result<(), EventLoopWithTimerError> {
        let event = EventLoopWithTimer::event(self.key, wants)?;
        self.poller
            .modify_with_mode(&self.fd, event, PollMode::Level)
            .map_err(EventLoopWithTimerError::Modify)?;
        Ok(())
    }
}

impl Drop for EventLoopAwareFd {
    fn drop(&mut self) {
        let _ = self.poller.delete(&self.fd);
    }
}

impl AsFd for EventLoopAwareFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
