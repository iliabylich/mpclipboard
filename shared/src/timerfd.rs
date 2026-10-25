use core::time::Duration;
use rustix::{
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    fs::Timespec,
    io::Errno,
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, timerfd_create,
        timerfd_settime,
    },
};

#[derive(Debug)]
#[must_use]
pub(crate) struct Timerfd {
    fd: OwnedFd,
}

impl Timerfd {
    pub(crate) fn new() -> Result<Self, Errno> {
        let fd = timerfd_create(
            TimerfdClockId::Monotonic,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;
        Ok(Self { fd })
    }

    pub(crate) fn set(&self, timeout: Duration) -> Result<(), Errno> {
        let timeout = timeout.max(Duration::from_nanos(1));
        timerfd_settime(
            &self.fd,
            TimerfdTimerFlags::empty(),
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: i64::try_from(timeout.as_secs()).unwrap_or(i64::MAX),
                    tv_nsec: timeout.subsec_nanos().into(),
                },
            },
        )?;
        Ok(())
    }
}

impl AsFd for Timerfd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Timerfd {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
