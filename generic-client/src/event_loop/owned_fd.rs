use crate::event_loop::EventLoop;
use std::{
    net::TcpStream,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd as _, OwnedFd},
    rc::Rc,
};

pub(crate) struct EventLoopAwareOwnedFd {
    fd: OwnedFd,
    event_loop: Rc<EventLoop>,
}
impl EventLoopAwareOwnedFd {
    pub(crate) fn new(fd: OwnedFd, event_loop: Rc<EventLoop>) -> Self {
        Self { fd, event_loop }
    }

    pub(crate) fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.event_loop)
    }

    pub(crate) fn into_tcp_stream(self) -> TcpStream {
        let stream = self.fd.as_raw_fd();
        std::mem::forget(self);
        unsafe { TcpStream::from_raw_fd(stream) }
    }
}

impl Drop for EventLoopAwareOwnedFd {
    fn drop(&mut self) {
        log::trace!("EventLoopAwareOwnedFd::drop()");
        self.event_loop.remove(self.as_raw_fd());
    }
}

impl AsRawFd for EventLoopAwareOwnedFd {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsFd for EventLoopAwareOwnedFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd.as_raw_fd()) }
    }
}
