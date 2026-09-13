use super::{Diff, EventLoopResult, FdState};
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

    pub fn new() -> Result<Self, EpollError> {
        let epoll_fd = epoll::create(epoll::CreateFlags::CLOEXEC)?;

        let this = Self {
            epoll_fd,
            timer: Timerfd::new()?,
            fd: FdState::new(),
        };
        this.add_timer()?;

        Ok(this)
    }

    pub fn sync(&mut self, wants: Option<(BorrowedFd<'_>, Wants)>) -> Result<(), EpollError> {
        match self.fd.transition(wants) {
            Diff::Add { fd, wants } => {
                self.add(unsafe { BorrowedFd::borrow_raw(fd) }, Self::FD_ID, wants)?;
            }
            Diff::Delete { fd } => {
                self.delete(unsafe { BorrowedFd::borrow_raw(fd) });
            }
            Diff::Modify { fd, wants } => {
                self.modify(unsafe { BorrowedFd::borrow_raw(fd) }, Self::FD_ID, wants)?;
            }
            Diff::Replace {
                prevfd,
                newfd,
                wants,
            } => {
                self.delete(unsafe { BorrowedFd::borrow_raw(prevfd) });
                self.add(unsafe { BorrowedFd::borrow_raw(newfd) }, Self::FD_ID, wants)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    pub fn drain_events_without_waiting(&mut self) -> Result<EventLoopResult, EpollError> {
        let mut events = [MaybeUninit::uninit(); 4];
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let (events, _) = epoll::wait(&self.epoll_fd, &mut events, Some(&timeout))?;

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events {
            match event.data.u64() {
                Self::TIMER_ID => {
                    let time = self.drain_timer()?;
                    out.time = Some(time);
                }

                Self::FD_ID => {
                    let flags = event.flags;
                    out.fd = Some((
                        flags.contains(epoll::EventFlags::IN),
                        flags.contains(epoll::EventFlags::OUT),
                        flags.intersects(
                            epoll::EventFlags::ERR
                                | epoll::EventFlags::HUP
                                | epoll::EventFlags::RDHUP,
                        ),
                    ));
                }

                _ => {
                    let id = event.data.u64();
                    return Err(EpollError::UnknownEvent { id });
                }
            }
        }

        Ok(out)
    }

    fn add(&self, fd: BorrowedFd<'_>, id: u64, wants: Wants) -> Result<(), EpollError> {
        epoll::add(
            &self.epoll_fd,
            fd,
            epoll::EventData::new_u64(id),
            Self::event_flags(wants),
        )?;
        Ok(())
    }

    fn delete(&self, fd: BorrowedFd<'_>) {
        let _ = epoll::delete(&self.epoll_fd, fd);
    }

    fn modify(&self, fd: BorrowedFd<'_>, id: u64, wants: Wants) -> Result<(), EpollError> {
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

    fn add_timer(&self) -> Result<(), EpollError> {
        epoll::add(
            &self.epoll_fd,
            &self.timer,
            epoll::EventData::new_u64(Self::TIMER_ID),
            epoll::EventFlags::IN,
        )?;
        Ok(())
    }

    fn drain_timer(&mut self) -> Result<u64, EpollError> {
        Ok(self.timer.read()?)
    }
}

#[derive(Debug)]
pub enum EpollError {
    Errno(Errno),
    UnknownEvent { id: u64 },
}

impl core::fmt::Display for EpollError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Errno(errno) => write!(f, "{errno}"),
            Self::UnknownEvent { id } => write!(f, "unknown epoll event {id}"),
        }
    }
}

impl core::error::Error for EpollError {}

impl From<Errno> for EpollError {
    fn from(error: Errno) -> Self {
        Self::Errno(error)
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
