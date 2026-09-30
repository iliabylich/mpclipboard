use super::{Diff, Epoch, EventLoopError, EventLoopResult, FdState};
use crate::Wants;
use core::{ptr, time::Duration};
use rustix::{event::kqueue as kq, io::Errno};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};

pub struct EventLoop {
    kqueue_fd: OwnedFd,
    time: u64,
    fd: FdState,
}

impl EventLoop {
    const TIMER_ID: isize = 1;
    const FD_ID: usize = 2;
    const INITIAL_TIMER_ID: isize = 3;

    pub fn new() -> Result<Self, EventLoopError> {
        let kqueue_fd = kq::kqueue().map_err(EventLoopError::Create)?;

        let this = Self {
            kqueue_fd,
            time: Self::now()?,
            fd: FdState::new(),
        };
        this.add_timer().map_err(EventLoopError::AddTimer)?;

        Ok(this)
    }

    pub fn sync(
        &mut self,
        wants: Option<(BorrowedFd<'_>, Epoch, Wants)>,
    ) -> Result<(), EventLoopError> {
        match self.fd.transition(wants) {
            Diff::Add { fd, wants } => {
                self.add(fd, wants).map_err(EventLoopError::Sync)?;
            }
            Diff::Delete { fd } => {
                self.delete(fd);
            }
            Diff::Modify { fd, wants } => {
                self.modify(fd, wants).map_err(EventLoopError::Sync)?;
            }
            Diff::Replace {
                prevfd,
                newfd,
                wants,
            } => {
                self.delete(prevfd);
                self.add(newfd, wants).map_err(EventLoopError::Sync)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    pub fn drain_events_without_waiting(&mut self) -> Result<EventLoopResult, EventLoopError> {
        let mut events = [Self::empty_event(); 4];
        let len = unsafe { kq::kevent(&self.kqueue_fd, &[], &mut events, Some(Duration::ZERO)) }
            .map_err(EventLoopError::Wait)?;

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events.iter().take(len) {
            match (event.filter(), event.udata() as usize) {
                (kq::EventFilter::Timer { ident, .. }, _) if ident == Self::TIMER_ID => {
                    out.time = Some(self.drain_timer(event));
                }
                (kq::EventFilter::Timer { ident, .. }, _) if ident == Self::INITIAL_TIMER_ID => {
                    out.time = Some(self.time);
                }
                (kq::EventFilter::Read(_) | kq::EventFilter::Write(_), tag)
                    if tag == Self::FD_ID =>
                {
                    let filter = event.filter();
                    let flags = event.flags();
                    let (mut readable, mut writable, mut has_error) =
                        out.fd.unwrap_or((false, false, false));

                    readable |= matches!(filter, kq::EventFilter::Read(_));
                    writable |= matches!(filter, kq::EventFilter::Write(_));
                    has_error |= flags.intersects(kq::EventFlags::ERROR | kq::EventFlags::EOF);

                    out.fd = Some((readable, writable, has_error));
                }
                _ => unreachable!("only timer and fd events are ever registered"),
            }
        }

        Ok(out)
    }

    fn empty_event() -> kq::Event {
        kq::Event::new(
            kq::EventFilter::Timer {
                ident: 0,
                timer: None,
            },
            kq::EventFlags::empty(),
            ptr::null_mut(),
        )
    }

    fn add(&self, fd: RawFd, wants: Wants) -> Result<(), Errno> {
        self.update_fd(fd, wants, kq::EventFlags::ADD | kq::EventFlags::ENABLE)
    }

    fn delete(&self, fd: RawFd) {
        self.delete_filter(kq::EventFilter::Read(fd));
        self.delete_filter(kq::EventFilter::Write(fd));
    }

    fn modify(&self, fd: RawFd, wants: Wants) -> Result<(), Errno> {
        self.delete(fd);
        self.add(fd, wants)
    }

    fn update_fd(&self, fd: RawFd, wants: Wants, flags: kq::EventFlags) -> Result<(), Errno> {
        let read = Self::event(kq::EventFilter::Read(fd), flags);
        let write = Self::event(kq::EventFilter::Write(fd), flags);

        match wants {
            Wants::ReadWrite => self.kevent(&[read, write]),
            Wants::Read => self.kevent(&[read]),
            Wants::Write => self.kevent(&[write]),
        }
    }

    fn delete_filter(&self, filter: kq::EventFilter) {
        let event = Self::event(filter, kq::EventFlags::DELETE);
        let _ = self.kevent(&[event]);
    }

    fn event(filter: kq::EventFilter, flags: kq::EventFlags) -> kq::Event {
        kq::Event::new(filter, flags, Self::FD_ID as *mut _)
    }

    fn kevent(&self, events: &[kq::Event]) -> Result<(), Errno> {
        let mut out: [kq::Event; 0] = [];
        unsafe { kq::kevent(&self.kqueue_fd, events, &mut out, Some(Duration::ZERO))? };
        Ok(())
    }

    fn add_timer(&self) -> Result<(), Errno> {
        let periodic = kq::Event::new(
            kq::EventFilter::Timer {
                ident: Self::TIMER_ID,
                timer: Some(Duration::from_secs(1)),
            },
            kq::EventFlags::ADD | kq::EventFlags::ENABLE,
            ptr::null_mut(),
        );
        let initial = kq::Event::new(
            kq::EventFilter::Timer {
                ident: Self::INITIAL_TIMER_ID,
                timer: Some(Duration::from_nanos(1)),
            },
            kq::EventFlags::ADD | kq::EventFlags::ENABLE | kq::EventFlags::ONESHOT,
            ptr::null_mut(),
        );
        self.kevent(&[periodic, initial])
    }

    fn drain_timer(&mut self, event: &kq::Event) -> u64 {
        let count = u64::try_from(event.data()).unwrap_or(1).max(1);
        let time = self
            .time
            .checked_add(count)
            .unwrap_or_else(|| unreachable!("seconds since 1970 never overflow u64"));
        self.time = time;
        self.time
    }

    fn now() -> Result<u64, EventLoopError> {
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| EventLoopError::ClockBeforeUnixEpoch)?
            .as_secs())
    }
}

impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.kqueue_fd.as_fd()
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.kqueue_fd.as_raw_fd()
    }
}
