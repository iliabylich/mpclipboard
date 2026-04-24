use polling::{Event, Events, PollMode, Poller};
use std::{
    cell::Cell,
    os::fd::{AsRawFd, BorrowedFd, OwnedFd},
    rc::Rc,
};

pub(crate) struct EventLoop {
    poller: Poller,
    ticks: Cell<u64>,
    #[allow(dead_code)]
    timerfd: Option<OwnedFd>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EventLoopEvent {
    pub(crate) tick: Option<u64>,
    pub(crate) ws: Option<(bool, bool)>,
}

impl EventLoop {
    const TIMER_ID: u32 = 1;
    const WS_ID: u32 = 2;

    pub(crate) fn new() -> Rc<Self> {
        let mut this = Self {
            poller: Poller::new().expect("bug: failed to instantiate event loop"),
            ticks: Cell::new(0),
            timerfd: None,
        };
        log::trace!("Supports edge: {}", this.poller.supports_edge());
        log::trace!("Supports level: {}", this.poller.supports_level());
        this.add_timer();
        Rc::new(this)
    }

    #[cfg(target_os = "macos")]
    fn add_timer(&mut self) {
        use polling::os::kqueue::{PollerKqueueExt, Timer};
        self.poller
            .add_filter(
                Timer {
                    id: 2,
                    timeout: std::time::Duration::from_secs(1),
                },
                Self::TIMER_ID as usize,
                PollMode::Level,
            )
            .expect("bug: failed to add timer");
    }

    #[cfg(target_os = "macos")]
    fn drain_timer(&self) {}

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn add_timer(&mut self) {
        use rustix::time::{
            Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
            timerfd_settime,
        };

        let timerfd = timerfd_create(TimerfdClockId::Monotonic, TimerfdFlags::NONBLOCK)
            .expect("bug: failed to create timerfd");

        timerfd_settime(
            &timerfd,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 10,
                    tv_nsec: 0,
                },
            },
        )
        .expect("bug: failed to configure timer");

        unsafe {
            self.poller
                .add_with_mode(
                    &timerfd,
                    Event::new(Self::TIMER_ID as usize, true, false),
                    PollMode::Level,
                )
                .expect("bug: failed to add timer to epoll")
        };

        self.timerfd = Some(timerfd);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn drain_timer(&self) {
        let mut buf = [0_u8; 8];
        let bytes_read = rustix::io::read(
            self.timerfd.as_ref().expect("bug: timerfd isn't set"),
            &mut buf,
        )
        .expect("bug: failed to read from timer");
        assert_eq!(bytes_read, 8);
    }

    pub(crate) fn add(&self, fd: i32, readable: bool, writable: bool) {
        unsafe {
            self.poller
                .add_with_mode(
                    fd,
                    Event::new(Self::WS_ID as usize, readable, writable),
                    PollMode::Level,
                )
                .expect("bug: failed to add FD to Poller")
        }
    }

    pub(crate) fn remove(&self, fd: i32) {
        if let Err(err) = self.poller.delete(unsafe { BorrowedFd::borrow_raw(fd) }) {
            log::error!("failed to delete FD from Poller: {err:?}")
        }
    }

    pub(crate) fn modify(&self, fd: i32, readable: bool, writable: bool) {
        self.poller
            .modify_with_mode(
                unsafe { BorrowedFd::borrow_raw(fd) },
                Event::new(Self::WS_ID as usize, readable, writable),
                PollMode::Level,
            )
            .expect("bug: failed to modify FD in Poller")
    }

    pub(crate) fn read(&self) -> EventLoopEvent {
        let mut events = Events::new();

        self.poller
            .wait(&mut events, None)
            .expect("bug: failed to read");

        let mut tick = None;
        let mut ws = None;

        for event in events.iter() {
            match event.key as u32 {
                Self::TIMER_ID => {
                    self.ticks.set(self.ticks.get() + 1);
                    self.drain_timer();
                    tick = Some(self.ticks.get());
                }

                Self::WS_ID => ws = Some((event.readable, event.writable)),

                _ => {
                    panic!("bug: unknown event: {event:?}")
                }
            }
        }

        EventLoopEvent { tick, ws }
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.poller.as_raw_fd()
    }
}
