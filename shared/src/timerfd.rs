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
pub struct Timerfd {
    fd: OwnedFd,
    time: u64,
}

impl Timerfd {
    pub fn new() -> Result<Self, Errno> {
        let fd = timerfd_create(
            TimerfdClockId::Monotonic,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;

        timerfd_settime(
            &fd,
            TimerfdTimerFlags::empty(),
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 0,
                    tv_nsec: 1,
                },
            },
        )?;

        Ok(Self { fd, time: 0 })
    }

    pub fn read(&mut self) -> Result<(), Errno> {
        let mut buf = [0u8; 8];
        let len = rustix::io::read(&self.fd, &mut buf)?;
        debug_assert_eq!(len, 8);

        let d = u64::from_le_bytes(buf);
        self.time = self.time.checked_add(d).ok_or(Errno::OVERFLOW)?;

        Ok(())
    }

    #[must_use]
    pub(crate) const fn time(&self) -> u64 {
        self.time
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

#[derive(Debug)]
#[must_use]
pub(crate) struct OneshotTimerfd {
    fd: OwnedFd,
}

impl OneshotTimerfd {
    pub(crate) fn new() -> Result<Self, Errno> {
        let fd = timerfd_create(
            TimerfdClockId::Monotonic,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;
        Ok(Self { fd })
    }

    pub(crate) fn set(&self, timeout: Option<Duration>) -> Result<(), Errno> {
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let it_value = match timeout {
            Some(timeout) => {
                let timeout = timeout.max(Duration::from_nanos(1));
                Timespec {
                    tv_sec: i64::try_from(timeout.as_secs()).unwrap_or(i64::MAX),
                    tv_nsec: timeout.subsec_nanos().into(),
                }
            }
            None => zero,
        };

        timerfd_settime(
            &self.fd,
            TimerfdTimerFlags::empty(),
            &Itimerspec {
                it_interval: zero,
                it_value,
            },
        )?;
        Ok(())
    }
}

impl AsFd for OneshotTimerfd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for OneshotTimerfd {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
