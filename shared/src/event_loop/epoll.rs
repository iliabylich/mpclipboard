use super::{Diff, Epoch, EventLoopError, EventLoopFdResult, EventLoopResult, FdState};
use crate::{Timerfd, Wants};
use core::mem::MaybeUninit;
use rustix::{
    event::epoll,
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    fs::Timespec,
    io::Errno,
};

pub struct EventLoop {
    epoll_fd: OwnedFd,
    timer: Timerfd,
    fd: FdState,
}

impl EventLoop {
    const TIMER_ID: u64 = 1;
    const FD_ID: u64 = 2;

    pub fn new() -> Result<Self, EventLoopError> {
        let epoll_fd =
            epoll::create(epoll::CreateFlags::CLOEXEC).map_err(EventLoopError::Create)?;

        let this = Self {
            epoll_fd,
            timer: Timerfd::new().map_err(EventLoopError::CreateTimer)?,
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
                self.add(fd, Self::FD_ID, wants)
                    .map_err(EventLoopError::Sync)?;
            }
            Diff::Modify { fd, wants } => {
                self.modify(fd, Self::FD_ID, wants)
                    .map_err(EventLoopError::Sync)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    pub fn drain_events_without_waiting(&mut self) -> Result<EventLoopResult, EventLoopError> {
        let mut events = [MaybeUninit::uninit(); 4];
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let (events, _) = epoll::wait(&self.epoll_fd, &mut events, Some(&timeout))
            .map_err(EventLoopError::Wait)?;

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events {
            match event.data.u64() {
                Self::TIMER_ID => {
                    let time = self.timer.read().map_err(EventLoopError::ReadTimer)?;
                    out.time = Some(time);
                }

                Self::FD_ID => {
                    let flags = event.flags;
                    out.fd = Some(EventLoopFdResult {
                        readable: flags.contains(epoll::EventFlags::IN),
                        writable: flags.contains(epoll::EventFlags::OUT),
                        has_error: flags.intersects(
                            epoll::EventFlags::ERR
                                | epoll::EventFlags::HUP
                                | epoll::EventFlags::RDHUP,
                        ),
                    });
                }

                _ => unreachable!("only timer and fd events are ever registered"),
            }
        }

        Ok(out)
    }

    fn add(&self, fd: BorrowedFd<'_>, id: u64, wants: Wants) -> Result<(), Errno> {
        epoll::add(
            &self.epoll_fd,
            fd,
            epoll::EventData::new_u64(id),
            Self::event_flags(wants),
        )?;
        Ok(())
    }

    fn modify(&self, fd: BorrowedFd<'_>, id: u64, wants: Wants) -> Result<(), Errno> {
        epoll::modify(
            &self.epoll_fd,
            fd,
            epoll::EventData::new_u64(id),
            Self::event_flags(wants),
        )?;
        Ok(())
    }

    fn event_flags(wants: Wants) -> epoll::EventFlags {
        (match wants {
            Wants::Read => epoll::EventFlags::IN,
            Wants::Write => epoll::EventFlags::OUT,
            Wants::ReadWrite => epoll::EventFlags::IN | epoll::EventFlags::OUT,
        }) | epoll::EventFlags::RDHUP
    }

    fn add_timer(&self) -> Result<(), Errno> {
        epoll::add(
            &self.epoll_fd,
            &self.timer,
            epoll::EventData::new_u64(Self::TIMER_ID),
            epoll::EventFlags::IN,
        )?;
        Ok(())
    }
}

impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.epoll_fd.as_fd()
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd.as_raw_fd()
    }
}
