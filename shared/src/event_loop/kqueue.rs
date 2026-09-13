use super::{Diff, EventLoopResult, FdState};
use crate::Wants;
use core::{ptr, time::Duration};
use rustix::event::kqueue as kq;
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

    pub fn new() -> std::io::Result<Self> {
        let kqueue_fd = kq::kqueue()?;

        let this = Self {
            kqueue_fd,
            time: Self::now(),
            fd: FdState::new(),
        };
        this.add_timer()?;

        Ok(this)
    }

    pub fn sync(&mut self, wants: Option<(RawFd, Wants)>) -> std::io::Result<()> {
        match self.fd.transition(wants) {
            Diff::Add { fd, wants } => {
                self.add(fd, wants)?;
            }
            Diff::Delete { fd } => {
                self.delete(fd);
            }
            Diff::Modify { fd, wants } => {
                self.modify(fd, wants)?;
            }
            Diff::Replace {
                prevfd,
                newfd,
                wants,
            } => {
                self.delete(prevfd);
                self.add(newfd, wants)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    pub fn drain_events_without_waiting(&mut self) -> std::io::Result<EventLoopResult> {
        let mut events = [Self::empty_event(); 4];
        let len = unsafe { kq::kevent(&self.kqueue_fd, &[], &mut events, Some(Duration::ZERO))? };

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events.iter().take(len) {
            match (event.filter(), event.udata() as usize) {
                (kq::EventFilter::Timer { ident, .. }, _) if ident == Self::TIMER_ID => {
                    out.time = Some(self.drain_timer(event)?);
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
                _ => {
                    return Err(std::io::Error::other("unknown event"));
                }
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

    fn add(&self, fd: RawFd, wants: Wants) -> std::io::Result<()> {
        self.update_fd(fd, wants, kq::EventFlags::ADD | kq::EventFlags::ENABLE)
    }

    fn delete(&self, fd: RawFd) {
        self.delete_filter(kq::EventFilter::Read(fd));
        self.delete_filter(kq::EventFilter::Write(fd));
    }

    fn modify(&self, fd: RawFd, wants: Wants) -> std::io::Result<()> {
        self.delete(fd);
        self.add(fd, wants)
    }

    fn update_fd(&self, fd: RawFd, wants: Wants, flags: kq::EventFlags) -> std::io::Result<()> {
        let read = Self::event(kq::EventFilter::Read(fd), flags);
        let write = Self::event(kq::EventFilter::Write(fd), flags);

        match (wants.wants_read(), wants.wants_write()) {
            (true, true) => self.kevent(&[read, write]),
            (true, false) => self.kevent(&[read]),
            (false, true) => self.kevent(&[write]),
            (false, false) => unreachable!("Wants always wants at least one event"),
        }
    }

    fn delete_filter(&self, filter: kq::EventFilter) {
        let event = Self::event(filter, kq::EventFlags::DELETE);
        let _ = self.kevent(&[event]);
    }

    fn event(filter: kq::EventFilter, flags: kq::EventFlags) -> kq::Event {
        kq::Event::new(filter, flags, Self::FD_ID as *mut _)
    }

    fn kevent(&self, events: &[kq::Event]) -> std::io::Result<()> {
        let mut out: [kq::Event; 0] = [];
        unsafe { kq::kevent(&self.kqueue_fd, events, &mut out, Some(Duration::ZERO))? };
        Ok(())
    }

    fn add_timer(&self) -> std::io::Result<()> {
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

    fn drain_timer(&mut self, event: &kq::Event) -> std::io::Result<u64> {
        let count = u64::try_from(event.data()).unwrap_or(1).max(1);
        self.time = self
            .time
            .checked_add(count)
            .ok_or_else(|| std::io::Error::other("timer overflow"))?;

        Ok(self.time)
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| unreachable!("time goes backwards"))
            .as_secs()
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
