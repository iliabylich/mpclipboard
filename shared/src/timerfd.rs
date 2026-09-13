use rustix::{
    fs::Timespec,
    io::Errno,
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, timerfd_create,
        timerfd_settime,
    },
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

#[derive(Debug)]
#[must_use]
pub struct Timerfd {
    fd: OwnedFd,
    time: u64,
}

impl Timerfd {
    pub fn new() -> Result<Self, Errno> {
        let fd = timerfd_create(
            TimerfdClockId::Realtime,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;

        timerfd_settime(
            &fd,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
            },
        )?;

        Ok(Self { fd, time: 0 })
    }

    pub fn read(&mut self) -> Result<u64, Errno> {
        let mut buf = [0u8; 8];
        let len = rustix::io::read(&self.fd, &mut buf)?;
        debug_assert_eq!(len, 8);

        let d = u64::from_le_bytes(buf);
        self.time = self.time.checked_add(d).ok_or(Errno::OVERFLOW)?;

        Ok(self.time)
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
