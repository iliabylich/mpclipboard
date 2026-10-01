use super::{Diff, Epoch, EventLoopError, EventLoopResult, FdState};
use crate::Wants;
use nix::sys::{
    event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue},
    time::TimeSpec,
};
use rustix::io::Errno;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

pub struct EventLoop {
    kqueue: Kqueue,
    time: u64,
    fd: FdState,
}

impl EventLoop {
    const TIMER_ID: usize = 1;
    const INITIAL_TIMER_ID: usize = 2;

    pub fn new() -> Result<Self, EventLoopError> {
        let kqueue = Kqueue::new()
            .map_err(errno)
            .map_err(EventLoopError::Create)?;

        let this = Self {
            kqueue,
            time: 0,
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
                self.add(fd.as_raw_fd(), wants)
                    .map_err(EventLoopError::Sync)?;
            }
            Diff::Modify { fd, wants } => {
                self.modify(fd.as_raw_fd(), wants)
                    .map_err(EventLoopError::Sync)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    pub fn drain_events_without_waiting(&mut self) -> Result<EventLoopResult, EventLoopError> {
        let mut events = [Self::empty_event(); 4];
        let len = self
            .kevent(&[], &mut events)
            .map_err(EventLoopError::Wait)?;

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events.iter().take(len) {
            match (event.filter(), event.ident()) {
                (Ok(EventFilter::EVFILT_TIMER), Self::TIMER_ID) => {
                    out.time = Some(self.drain_timer(event));
                }
                (Ok(EventFilter::EVFILT_TIMER), Self::INITIAL_TIMER_ID) => {
                    out.time = Some(self.time);
                }
                (Ok(filter @ (EventFilter::EVFILT_READ | EventFilter::EVFILT_WRITE)), _) => {
                    let (mut readable, mut writable, mut has_error) =
                        out.fd.unwrap_or((false, false, false));

                    readable |= filter == EventFilter::EVFILT_READ;
                    writable |= filter == EventFilter::EVFILT_WRITE;
                    has_error |= event
                        .flags()
                        .intersects(EvFlags::EV_ERROR | EvFlags::EV_EOF);

                    out.fd = Some((readable, writable, has_error));
                }
                _ => unreachable!("only timer and fd events are ever registered"),
            }
        }

        Ok(out)
    }

    fn empty_event() -> KEvent {
        KEvent::new(
            0,
            EventFilter::EVFILT_TIMER,
            EvFlags::empty(),
            FilterFlag::empty(),
            0,
            0,
        )
    }

    fn add(&self, fd: RawFd, wants: Wants) -> Result<(), Errno> {
        self.update_fd(fd, wants, EvFlags::EV_ADD | EvFlags::EV_ENABLE)
    }

    fn delete(&self, fd: RawFd) {
        self.delete_filter(fd, EventFilter::EVFILT_READ);
        self.delete_filter(fd, EventFilter::EVFILT_WRITE);
    }

    fn modify(&self, fd: RawFd, wants: Wants) -> Result<(), Errno> {
        self.delete(fd);
        self.add(fd, wants)
    }

    fn update_fd(&self, fd: RawFd, wants: Wants, flags: EvFlags) -> Result<(), Errno> {
        let read = Self::event(fd, EventFilter::EVFILT_READ, flags);
        let write = Self::event(fd, EventFilter::EVFILT_WRITE, flags);

        match wants {
            Wants::ReadWrite => self.apply(&[read, write]),
            Wants::Read => self.apply(&[read]),
            Wants::Write => self.apply(&[write]),
        }
    }

    fn delete_filter(&self, fd: RawFd, filter: EventFilter) {
        let event = Self::event(fd, filter, EvFlags::EV_DELETE);
        let _ = self.apply(&[event]);
    }

    fn event(fd: RawFd, filter: EventFilter, flags: EvFlags) -> KEvent {
        let ident =
            usize::try_from(fd).unwrap_or_else(|_| unreachable!("open fds are never negative"));
        KEvent::new(ident, filter, flags, FilterFlag::empty(), 0, 0)
    }

    fn apply(&self, changes: &[KEvent]) -> Result<(), Errno> {
        let mut out: [KEvent; 0] = [];
        self.kevent(changes, &mut out)?;
        Ok(())
    }

    fn kevent(&self, changes: &[KEvent], out: &mut [KEvent]) -> Result<usize, Errno> {
        let timeout = *TimeSpec::new(0, 0).as_ref();
        self.kqueue
            .kevent(changes, out, Some(timeout))
            .map_err(errno)
    }

    fn add_timer(&self) -> Result<(), Errno> {
        let periodic = KEvent::new(
            Self::TIMER_ID,
            EventFilter::EVFILT_TIMER,
            EvFlags::EV_ADD | EvFlags::EV_ENABLE,
            FilterFlag::NOTE_SECONDS,
            1,
            0,
        );
        let initial = KEvent::new(
            Self::INITIAL_TIMER_ID,
            EventFilter::EVFILT_TIMER,
            EvFlags::EV_ADD | EvFlags::EV_ENABLE | EvFlags::EV_ONESHOT,
            FilterFlag::NOTE_NSECONDS,
            1,
            0,
        );
        self.apply(&[periodic, initial])
    }

    fn drain_timer(&mut self, event: &KEvent) -> u64 {
        let count = u64::try_from(event.data()).unwrap_or(1).max(1);
        let time = self
            .time
            .checked_add(count)
            .unwrap_or_else(|| unreachable!("seconds of uptime never overflow u64"));
        self.time = time;
        self.time
    }
}

const fn errno(errno: nix::errno::Errno) -> Errno {
    Errno::from_raw_os_error(errno as i32)
}

impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.kqueue.as_fd()
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.kqueue.as_fd().as_raw_fd()
    }
}
